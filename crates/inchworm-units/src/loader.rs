use std::collections::{BTreeMap, HashMap, HashSet};

use inchworm_dimensions::{DimRegistry, DimensionError};

use crate::{
    BaseDef, DerivedDef, PointDef, Unit, UnitError, UnitRegistry, fold::lookup_key,
    graph::topological_order, parse::idents,
};

pub(crate) const SUPPORTED_SCHEMA: u32 = 1;

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UnitFile {
    /// Format version; checked against [`SUPPORTED_SCHEMA`].
    pub schema: u32,
    /// The `[registry]` table.
    pub registry: RegistryMeta,
    /// `[prefix]` entries.
    #[serde(default)]
    pub prefix: BTreeMap<String, PrefixEntry>,
    /// `[base]` entries.
    #[serde(default)]
    pub base: BTreeMap<String, BaseEntry>,
    /// `[derived]` entries.
    #[serde(default)]
    pub derived: BTreeMap<String, DerivedEntry>,
    /// `[point]` entries.
    #[serde(default)]
    pub point: BTreeMap<String, PointEntry>,
}

/// The `[registry]` table: metadata for a freshly built registry.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct RegistryMeta {
    pub name: String,
    pub version: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrefixEntry {
    pub symbol: String,
    pub factor: f64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BaseEntry {
    pub symbol: String,
    pub dimension: String,
    #[serde(default)]
    pub prefixable: bool,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DerivedEntry {
    pub symbol: String,
    pub definition: String,
    #[serde(default)]
    pub prefixable: bool,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PointEntry {
    pub symbol: String,
    pub delta: String,
    pub offset: f64,
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// Deserializes `src` and checks its schema version.
fn parse_file(src: &str) -> Result<UnitFile, UnitError> {
    let file: UnitFile =
        toml::from_str(src).map_err(|e| UnitError::DefinitionFile(e.to_string()))?;
    if file.schema != SUPPORTED_SCHEMA {
        return Err(UnitError::DefinitionFile(format!(
            "unsupported schema {}; this version of inchworm-units only supports schema {SUPPORTED_SCHEMA}",
            file.schema
        )));
    }
    Ok(file)
}

fn base_def(registry: &UnitRegistry, name: &str, entry: &BaseEntry) -> Result<BaseDef, UnitError> {
    let Some(dimension) = registry.dims().get(&entry.dimension) else {
        return Err(UnitError::Dimension(DimensionError::UnknownDimension {
            name: entry.dimension.clone(),
            registry: registry.dims().name().into(),
        }));
    };
    let def = BaseDef::new(name, &entry.symbol, &dimension).prefixable(entry.prefixable);
    let def = match entry.prefix.as_deref() {
        Some(prefix) => def.prefixed(prefix),
        None => def,
    };
    let def = entry
        .aliases
        .iter()
        .fold(def, |def, alias| def.alias(alias));
    Ok(def)
}

fn derived_def(name: &str, entry: &DerivedEntry) -> DerivedDef {
    let def =
        DerivedDef::from_expr(name, &entry.symbol, &entry.definition).prefixable(entry.prefixable);
    entry
        .aliases
        .iter()
        .fold(def, |def, alias| def.alias(alias))
}

fn point_def(
    registry: &UnitRegistry,
    name: &str,
    entry: &PointEntry,
) -> Result<PointDef, UnitError> {
    let delta = match registry.parse(&entry.delta)? {
        Unit::Delta(delta) => delta,
        Unit::Point(point) => {
            return Err(UnitError::NotComposable {
                name: point.name().into(),
            });
        }
    };
    let def = PointDef::new(name, &entry.symbol, &delta, entry.offset);
    Ok(entry
        .aliases
        .iter()
        .fold(def, |def, alias| def.alias(alias)))
}

enum Pending<'a> {
    Derived(&'a DerivedEntry),
    Point(&'a PointEntry),
}

impl Pending<'_> {
    fn symbol(&self) -> &str {
        match self {
            Pending::Derived(e) => &e.symbol,
            Pending::Point(e) => &e.symbol,
        }
    }

    fn aliases(&self) -> &[String] {
        match self {
            Pending::Derived(e) => &e.aliases,
            Pending::Point(e) => &e.aliases,
        }
    }

    fn source(&self) -> &str {
        match self {
            Pending::Derived(e) => &e.definition,
            Pending::Point(e) => &e.delta,
        }
    }
}

/// The pending entry `ident` refers to, if any, following, `resolve_ident`'s order.
fn dependency<'a>(
    ident: &str,
    index: &HashMap<String, &'a str>,
    prefixes: &HashSet<String>,
    registry: &UnitRegistry,
) -> Option<&'a str> {
    if let Some(&name) = index.get(lookup_key(ident).as_ref()) {
        return Some(name);
    }
    if registry.get(ident).is_some() {
        return None;
    }
    ident
        .char_indices()
        .rev()
        .filter(|&(split, _)| split > 0)
        .find_map(|(split, _)| {
            let (prefix, stem) = ident.split_at(split);
            if !prefixes.contains(lookup_key(prefix).as_ref()) {
                return None;
            }
            index.get(lookup_key(stem).as_ref()).copied()
        })
}

/// Builds a new registry, owning `dims`, from the definition file `src`.
pub(crate) fn load_registry(src: &str, dims: DimRegistry) -> Result<UnitRegistry, UnitError> {
    let file = parse_file(src)?;
    let mut ureg = UnitRegistry::new_with_meta(&file.registry.name, &file.registry.version, dims);
    for (name, prefix) in &file.prefix {
        ureg.add_prefix(name, &prefix.symbol, prefix.factor)?;
    }
    for (name, base) in &file.base {
        ureg.add_base(base_def(&ureg, name, base)?)?;
    }
    let entries = file
        .derived
        .iter()
        .map(|(name, e)| (name, Pending::Derived(e)))
        .chain(file.point.iter().map(|(name, e)| (name, Pending::Point(e))));
    let mut pending: HashMap<&str, Pending> = HashMap::new();
    for (name, entry) in entries {
        if pending.insert(name, entry).is_some() {
            return Err(UnitError::DuplicateName {
                name: name.into(),
                registry: ureg.name().into(),
            });
        }
    }
    let mut index: HashMap<String, &str> = HashMap::new();
    for (&name, entry) in &pending {
        let idents = [name, entry.symbol()]
            .into_iter()
            .chain(entry.aliases().iter().map(String::as_str));
        for ident in idents {
            if let Some(other) = index.insert(lookup_key(ident).into_owned(), name)
                && other != name
            {
                return Err(UnitError::DuplicateName {
                    name: ident.into(),
                    registry: ureg.name().into(),
                });
            }
        }
    }
    let prefixes: HashSet<String> = file
        .prefix
        .iter()
        .flat_map(|(name, prefix)| [name.as_str(), prefix.symbol.as_str()])
        .map(|ident| lookup_key(ident).into_owned())
        .collect();
    let graph: HashMap<String, Vec<String>> = pending
        .iter()
        .map(|(&name, entry)| {
            let deps = idents(entry.source())
                .iter()
                .filter_map(|ident| dependency(ident, &index, &prefixes, &ureg))
                .map(str::to_string)
                .collect();
            (name.to_string(), deps)
        })
        .collect();
    for name in topological_order(&graph)? {
        match pending[name.as_str()] {
            Pending::Derived(entry) => {
                ureg.add_derived(derived_def(&name, entry))?;
            }
            Pending::Point(entry) => {
                ureg.add_point(point_def(&ureg, &name, entry)?)?;
            }
        }
    }
    Ok(ureg)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = r#"schema = 1

    [registry]
    name = "test-ureg"
    version = "0.1.0"
    "#;

    mod parse_file {
        use super::*;

        #[test]
        fn parses_keyed_sections() {
            let src = format!(
                r#"{HEADER}
                [prefix]
                kilo = {{ symbol = "k", factor = 1e3 }}

                [base]
                gram = {{ symbol = "g", dimension = "mass", prefixable = true, prefix = "kilo", aliases = ["gramme"] }}

                [derived]
                hour = {{ symbol = "h", definition = "3600 s", aliases = ["hr"] }}

                [point]
                degree_Celsius = {{ symbol = "degC", delta = "K", offset = 273.15, aliases = ["celsius"] }}
                "#
            );
            let file = parse_file(&src).unwrap();
            assert_eq!(file.registry.name, "test-ureg");
            assert_eq!(file.registry.version, "0.1.0");

            let kilo = &file.prefix["kilo"];
            assert_eq!(kilo.symbol, "k");
            assert_eq!(kilo.factor, 1e3);

            let gram = &file.base["gram"];
            assert_eq!(gram.symbol, "g");
            assert_eq!(gram.dimension, "mass");
            assert!(gram.prefixable);
            assert_eq!(gram.prefix.as_deref(), Some("kilo"));
            assert_eq!(gram.aliases, ["gramme"]);

            let hour = &file.derived["hour"];
            assert_eq!(hour.symbol, "h");
            assert_eq!(hour.definition, "3600 s");
            assert_eq!(hour.aliases, ["hr"]);

            let celsius = &file.point["degree_Celsius"];
            assert_eq!(celsius.symbol, "degC");
            assert_eq!(celsius.delta, "K");
            assert_eq!(celsius.offset, 273.15);
            assert_eq!(celsius.aliases, ["celsius"]);
        }

        #[test]
        fn optional_fields_default() {
            let src = format!(
                r#"{HEADER}
                [base]
                meter = {{ symbol = "m", dimension = "length" }}

                [derived]
                hour = {{ symbol = "h", definition = "3600 s" }}

                [point]
                degree_Celsius = {{ symbol = "degC", delta = "K", offset = 273.15 }}
                "#
            );
            let file = parse_file(&src).unwrap();
            let meter = &file.base["meter"];
            assert!(!meter.prefixable);
            assert!(meter.prefix.is_none());
            assert!(meter.aliases.is_empty());
            let hour = &file.derived["hour"];
            assert!(!hour.prefixable);
            assert!(hour.aliases.is_empty());
            assert!(file.point["degree_Celsius"].aliases.is_empty());
        }

        #[test]
        fn missing_sections_are_empty() {
            let file = parse_file(HEADER).unwrap();
            assert!(file.prefix.is_empty());
            assert!(file.base.is_empty());
            assert!(file.derived.is_empty());
            assert!(file.point.is_empty());
        }

        #[test]
        fn rejects_unknown_field() {
            let cases = [
                r#"[base]
                meter = { symbol = "m", dimension = "length", prefixible = true }"#,
                // The factor belongs in the definition string: there is no `scale` field.
                r#"[derived]
                hour = { symbol = "h", definition = "s", scale = 3600.0 }"#,
                r#"[point]
                degC = { symbol = "degC", delta = "K", offset = 273.15, origin = 0.0 }"#,
                // The Step 2.7 draft's single list of units is not a section.
                r#"[unit]
                meter = { symbol = "m" }"#,
            ];
            for body in cases {
                let src = format!("{HEADER}\n{body}\n");
                assert!(
                    matches!(parse_file(&src), Err(UnitError::DefinitionFile(_))),
                    "{body}"
                );
            }
        }

        #[test]
        fn rejects_missing_required_field() {
            let cases = [
                r#"[prefix]
                kilo = { symbol = "k" }"#, // no factor
                r#"[base]
                meter = { symbol = "m" }"#, // no dimension
                r#"[point]
                degC = { symbol = "degC", delta = "K" }"#, // no offset
            ];
            for body in cases {
                let src = format!("{HEADER}\n{body}\n");
                assert!(
                    matches!(parse_file(&src), Err(UnitError::DefinitionFile(_))),
                    "{body}"
                );
            }
        }

        #[test]
        fn rejects_missing_registry() {
            let src = r#"schema = 1
            "#;
            let err = parse_file(src).unwrap_err();
            assert!(matches!(&err, UnitError::DefinitionFile(_)));
        }

        #[test]
        fn rejects_unsupported_schema() {
            let src = r#"schema = 0

            [registry]
            name = "test-ureg"
            version = "0.1.0"
            "#;
            let UnitError::DefinitionFile(message) = parse_file(src).unwrap_err() else {
                panic!("not a DefinitionFile error")
            };
            assert!(message.contains("unsupported schema 0"));
        }
    }

    mod load_registry {
        use super::*;
        use crate::test_utils::{assert_close, assert_expands_to, errors_match};

        /// Dimensions `length`, `mass`, `time`, and `temperature`.
        fn dims() -> DimRegistry {
            let mut dims = DimRegistry::new("test-reg");
            for name in ["length", "mass", "time", "temperature"] {
                dims.add_base(name, None).unwrap();
            }
            dims
        }

        /// Loads `body` (the sections) under the shared header.
        fn load(body: &str) -> Result<UnitRegistry, UnitError> {
            load_registry(&format!("{HEADER}\n{body}\n"), dims())
        }

        /// The error loading `body` fails with.
        fn load_err(body: &str) -> UnitError {
            load(body).err().expect("the load should fail")
        }

        fn duplicate(name: &str) -> UnitError {
            UnitError::DuplicateName {
                name: name.into(),
                registry: "test-ureg".into(),
            }
        }

        #[test]
        fn build_registry_from_file() {
            let ureg = load(
                r#"
                [prefix]
                kilo = { symbol = "k", factor = 1e3 }

                [base]
                meter = { symbol = "m", dimension = "length", prefixable = true, aliases = ["metre"] }
                gram = { symbol = "g", dimension = "mass", prefixable = true, prefix = "kilo" }
                second = { symbol = "s", dimension = "time", prefixable = true }
                kelvin = { symbol = "K", dimension = "temperature" }

                [derived]
                joule = { symbol = "J", definition = "kg m^2 / s^2", prefixable = true }

                [point]
                degree_Celsius = { symbol = "degC", delta = "K", offset = 273.15 }
                "#,
            )
            .unwrap();
            assert_eq!(ureg.name(), "test-ureg");
            assert_eq!(ureg.version(), "0.1.0");
            assert_expands_to(&ureg, "metre", 1.0, "m");
            assert_expands_to(&ureg, "g", 1e-3, "kg");
            assert_expands_to(&ureg, "kJ", 1e3, "kg*m^2/s^2");
            let celsius = ureg.parse("degC").unwrap();
            let kelvin = ureg.parse("K").unwrap();
            assert_close(celsius.converter_to(&kelvin).unwrap().apply(0.0), 273.15);
        }

        #[test]
        fn resolves_forward_references() {
            // `hour` sorts before `minute`, but needs it.
            let ureg = load(
                r#"
                [base]
                second = { symbol = "s", dimension = "time" }

                [derived]
                hour = { symbol = "h", definition = "60 min" }
                minute = { symbol = "min", definition = "60 s" }
                "#,
            )
            .unwrap();
            assert_expands_to(&ureg, "h", 3600.0, "s");
        }

        #[test]
        fn forward_reference_is_not_shadowed_by_prefixed_form() {
            // Before `minute` exists, `min` would resolve as milli-inch: `hour` must wait for it.
            let ureg = load(
                r#"
                [prefix]
                milli = { symbol = "m", factor = 1e-3 }

                [base]
                second = { symbol = "s", dimension = "time" }
                inch = { symbol = "in", dimension = "length", prefixable = true }

                [derived]
                hour = { symbol = "h", definition = "60 min" }
                minute = { symbol = "min", definition = "60 s" }
                "#,
            )
            .unwrap();
            assert_expands_to(&ureg, "h", 3600.0, "s");
        }

        #[test]
        fn resolves_prefixed_forward_reference() {
            // `big` sorts before `foo`, and refers to it through a prefix.
            let ureg = load(
                r#"
                [prefix]
                kilo = { symbol = "k", factor = 1e3 }

                [base]
                meter = { symbol = "m", dimension = "length" }

                [derived]
                big = { symbol = "big", definition = "2 kfoo" }
                foo = { symbol = "foo", definition = "3 m", prefixable = true }
                "#,
            )
            .unwrap();
            assert_expands_to(&ureg, "big", 6000.0, "m");
        }

        #[test]
        fn rejects_cycles() {
            let cases = [
                (
                    r#"
                    [derived]
                    a = { symbol = "a", definition = "2 b" }
                    b = { symbol = "b", definition = "3 a" }
                    "#,
                    vec!["a", "b"],
                ),
                (
                    r#"
                    [derived]
                    c = { symbol = "c", definition = "2 c" }
                    "#,
                    vec!["c"],
                ),
            ];
            for (body, names) in cases {
                let actual = load_err(body);
                let expected = UnitError::CyclicDefinition {
                    names: names.into_iter().map(String::from).collect(),
                };
                assert!(errors_match(&actual, &expected), "{actual:?}");
            }
        }

        #[test]
        fn reports_unknown_unit() {
            let actual = load_err(
                r#"
                [derived]
                x = { symbol = "x", definition = "2 parsec" }
                "#,
            );
            let expected = UnitError::UnknownUnit {
                name: "parsec".into(),
                registry: "test-ureg".into(),
            };
            assert!(errors_match(&actual, &expected), "{actual:?}");
        }

        #[test]
        fn rejects_unknown_dimension() {
            let actual = load_err(
                r#"
                [base]
                meter = { symbol = "m", dimension = "nope" }
                "#,
            );
            let expected = UnitError::Dimension(DimensionError::UnknownDimension {
                name: "nope".into(),
                registry: "test-reg".into(),
            });
            assert!(errors_match(&actual, &expected), "{actual:?}");
        }

        #[test]
        fn rejects_duplicates_across_sections() {
            let cases = [
                // Caught by `add_derived`: the base is already registered.
                r#"
                [base]
                meter = { symbol = "m", dimension = "length" }

                [derived]
                meter = { symbol = "M2", definition = "2 m" }
                "#,
                // Caught by the loader: both are pending.
                r#"
                [base]
                kelvin = { symbol = "K", dimension = "temperature" }

                [derived]
                meter = { symbol = "M2", definition = "2 K" }

                [point]
                meter = { symbol = "M3", delta = "K", offset = 1.0 }
                "#,
            ];
            for body in cases {
                let actual = load_err(body);
                assert!(errors_match(&actual, &duplicate("meter")), "{actual:?}");
            }
        }

        #[test]
        fn rejects_identifier_shared_by_two_entries() {
            let actual = load_err(
                r#"
                [base]
                meter = { symbol = "m", dimension = "length" }

                [derived]
                a = { symbol = "s1", definition = "2 m" }
                b = { symbol = "s2", definition = "3 m", aliases = ["s1"] }
                "#,
            );
            assert!(errors_match(&actual, &duplicate("s1")), "{actual:?}");
        }

        #[test]
        fn rejects_point_delta_that_is_a_point() {
            let actual = load_err(
                r#"
                [base]
                kelvin = { symbol = "K", dimension = "temperature" }

                [point]
                degree_Celsius = { symbol = "degC", delta = "K", offset = 273.15 }
                weird = { symbol = "w", delta = "degC", offset = 1.0 }
                "#,
            );
            let expected = UnitError::NotComposable {
                name: "degree_Celsius".into(),
            };
            assert!(errors_match(&actual, &expected), "{actual:?}");
        }
    }
}
