//! TOML definition-file loading.
//!
//! Loading semantics:
//! parse the whole file → build the name-dependency graph → resolve topologically
//! (file order is irrelevant; forward references are allowed).
//!
//! Errors:
//! Cycles → `CyclicDefinition`; duplicates (within the file or vs
//! the registry) → `DuplicateName`.
//!
//! Loading into a non-empty registry is additive — this is how "customize the
//! standard registry from a file" works.

use std::collections::{BTreeMap, HashMap};

use crate::{DimRegistry, DimensionError, graph::topological_order, parser::extract_idents};

pub(crate) const SUPPORTED_SCHEMA: u32 = 1;

/// The deserialized shape of a definition TOML file/string.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DimFile {
    /// Format version; checked against [`SUPPORTED_SCHEMA`].
    pub schema: u32,
    /// The `[registry]` table. Required when building a new registry via
    /// [`load_registry`]; ignored when extending an existing one via
    /// [`extend_registry`].
    pub registry: Option<RegistryMeta>,
    /// `[base]` entries.
    #[serde(default)]
    pub base: BTreeMap<String, BaseEntry>,
    /// `[derived]` entries.
    #[serde(default)]
    pub derived: BTreeMap<String, DerivedEntry>,
    /// `[dimensionless]` entries.
    #[serde(default)]
    pub dimensionless: BTreeMap<String, DimensionlessEntry>,
}

/// The `[registry]` table: metadata for a freshly built registry.
#[derive(serde::Deserialize)]
pub(crate) struct RegistryMeta {
    pub name: String,
    pub version: String,
}

/// A `[base]` entry.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BaseEntry {
    #[serde(default)]
    pub symbol: Option<String>,
}

/// A `[derived]` entry.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DerivedEntry {
    pub definition: String,
    /// Alternate names registered for this entry's name via
    /// [`DimRegistry::add_alias`](crate::DimRegistry::add_alias).
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// A `[dimensionless]` entry.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DimensionlessEntry {
    pub definition: String,
    /// Alternate names registered for this entry's name via
    /// [`DimRegistry::add_alias`](crate::DimRegistry::add_alias).
    #[serde(default)]
    pub aliases: Vec<String>,
}

enum DefEntry {
    Base(BaseEntry),
    Derived(DerivedEntry),
    Dimensionless(DimensionlessEntry),
}

/// Builds a new [`DimRegistry`] from the source.
///
/// # Errors
///
/// Returns [`DimensionError::DefinitionFile`] if the TOML source cannot be deserialized correctly or if it does not contain the `[registry]` section.
/// Returns [`DimensionError::UnknownDimension`] if any dimension referenced by the source was never defined.
/// Returns [`DimensionError::DuplicateName`] if duplicate names are found in the source.
/// Returns [`DimensionError::CyclicDefinition`] if cyclic definitions are found in the source.
/// Returns [`DimensionError::NotDimensionless`] if a declared dimensionless dimension is not in fact dimensionless.
/// Returns [`DimensionError::Parse`] if a dimension expression in the source cannot be parsed.
/// Returns [`DimensionError::ExponentOverflow`] if any exponents in the source definitions incur overflow.
/// Returns [`DimensionError::ZeroDenominator`] if any exponents' denominators in the source definitions are zero.
pub(crate) fn load_registry(src: &str) -> Result<DimRegistry, DimensionError> {
    let file: DimFile =
        toml::from_str(src).map_err(|e| DimensionError::DefinitionFile(e.to_string()))?;
    let registry_meta = file.registry.as_ref().ok_or_else(|| {
        DimensionError::DefinitionFile(
            "missing [registry] section; a name is required to build a new registry".into(),
        )
    })?;
    let mut registry = DimRegistry::new_with_meta(&registry_meta.name, &registry_meta.version);
    load_entries(&mut registry, file)?;
    Ok(registry)
}

/// Extends the registry with the source definitions.
///
/// # Errors
///
/// Returns [`DimensionError::DefinitionFile`] if the TOML source cannot be deserialized correctly.
/// Returns [`DimensionError::UnknownDimension`] if any dimension referenced by the source was never defined.
/// Returns [`DimensionError::DuplicateName`] if duplicate names are found in the source.
/// Returns [`DimensionError::CyclicDefinition`] if cyclic definitions are found in the source.
/// Returns [`DimensionError::NotDimensionless`] if a declared dimensionless dimension is not in fact dimensionless.
/// Returns [`DimensionError::Parse`] if the information in the source cannot be parsed.
/// Returns [`DimensionError::ExponentOverflow`] if any exponents in the source definitions incur overflow.
/// Returns [`DimensionError::ZeroDenominator`] if any exponents' denominators in the source definitions are zero.
pub(crate) fn extend_registry(registry: &mut DimRegistry, src: &str) -> Result<(), DimensionError> {
    let file: DimFile =
        toml::from_str(src).map_err(|e| DimensionError::DefinitionFile(e.to_string()))?;
    load_entries(registry, file)?;
    Ok(())
}

/// Shared by [`load_registry`] and [`extend_registry`]: builds the dependency graph, sorts it,
/// and adds entries to `registry`.
fn load_entries(registry: &mut DimRegistry, file: DimFile) -> Result<(), DimensionError> {
    if file.schema != SUPPORTED_SCHEMA {
        return Err(DimensionError::DefinitionFile(format!(
            "unsupported schema {}; this version of inchworm-dimensions only supports schema {SUPPORTED_SCHEMA}",
            file.schema
        )));
    }
    let entries = file
        .base
        .into_iter()
        .map(|(name, e)| (name, DefEntry::Base(e)))
        .chain(
            file.derived
                .into_iter()
                .map(|(name, e)| (name, DefEntry::Derived(e))),
        )
        .chain(
            file.dimensionless
                .into_iter()
                .map(|(name, e)| (name, DefEntry::Dimensionless(e))),
        );
    let mut def_entries = HashMap::new();
    for (name, entry) in entries {
        if def_entries.insert(name.clone(), entry).is_some() {
            return Err(DimensionError::DuplicateName {
                name,
                registry: registry.name().into(),
            });
        }
    }
    let mut dep_graph: HashMap<String, Vec<String>> = HashMap::new();
    for (name, def_entry) in &def_entries {
        let definition = match def_entry {
            DefEntry::Base(_) => Vec::new(),
            DefEntry::Derived(d) => extract_idents(&d.definition)?,
            DefEntry::Dimensionless(d) => extract_idents(&d.definition)?,
        };
        dep_graph.insert(name.clone(), definition);
    }
    let sorted_dependencies = topological_order(&dep_graph)?;
    for name in sorted_dependencies {
        match def_entries
            .get(&name)
            .expect("name from topological sort must be a def_entries key")
        {
            DefEntry::Base(b) => {
                registry.add_base(&name, b.symbol.as_deref())?;
            }
            DefEntry::Derived(d) => {
                registry.add_derived_expr(&name, &d.definition)?;
                for alias in &d.aliases {
                    registry.add_alias(alias, &name)?;
                }
            }
            DefEntry::Dimensionless(d) => {
                registry.add_dimensionless_expr(&name, &d.definition)?;
                for alias in &d.aliases {
                    registry.add_alias(alias, &name)?;
                }
            }
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assert_exactly_eq, errors_match};

    mod load_registry {
        use super::*;

        #[test]
        fn builds_registry_from_minimal_file() {
            let src = r#"schema = 1

            [registry]
            name = "test-reg"
            version = "0.1.0"

            [base]
            base = { symbol = "B" }
            "#;
            let registry = load_registry(src).unwrap();
            assert_eq!(
                registry.name(),
                "test-reg",
                "registry name must match the one defined by source"
            );
            assert_eq!(
                registry.version(),
                "0.1.0",
                "registry version must match the one defined by source"
            );
            assert!(
                registry.get("base").is_some(),
                "source defined dimension should be in the registry"
            );
        }

        #[test]
        fn resolves_forward_reference() {
            let src = r#"schema = 1

            [registry]
            name = "test-reg"
            version = "0.0.0"

            [base]
            a = { symbol = "A" }

            [derived]
            c = { definition = "b" }
            b = { definition = "a" }
            "#;
            let registry = load_registry(src).unwrap();
            assert!(
                registry.get("c").is_some(),
                "load_registry should resolve forward reference"
            );
        }

        #[test]
        fn builds_valid_dimensionless_entry() {
            let src = r#"schema = 1

            [registry]
            name = "test-reg"
            version = "0.0.0"

            [base]
            base = { symbol = "B" }

            [dimensionless]
            dimensionless = { definition = "base / base" }
            "#;
            let registry = load_registry(src).unwrap();
            assert!(
                registry.get("dimensionless").is_some(),
                "load_registry should build valid dimensionless entry"
            );
        }

        #[test]
        fn wires_up_derived_entry_aliases() {
            let src = r#"schema = 1

            [registry]
            name = "test-reg"
            version = "0.0.0"

            [base]
            a = { symbol = "A" }

            [derived]
            b = { definition = "a", aliases = ["c"] }
            "#;
            let registry = load_registry(src).unwrap();
            assert_exactly_eq(&registry.get("c").unwrap(), &registry.get("b").unwrap());
        }

        #[test]
        fn wires_up_dimensionless_entry_aliases() {
            let src = r#"schema = 1

            [registry]
            name = "test-reg"
            version = "0.0.0"

            [base]
            a = { symbol = "A" }

            [dimensionless]
            b = { definition = "a / a", aliases = ["c"] }
            "#;
            let registry = load_registry(src).unwrap();
            assert_exactly_eq(&registry.get("c").unwrap(), &registry.get("b").unwrap());
        }

        #[test]
        fn rejects_invalid_dimensionless_entry() {
            let src = r#"schema = 1

            [registry]
            name = "test-reg"
            version = "0.0.0"

            [base]
            base = { symbol = "B" }

            [dimensionless]
            dimensionless = { definition = "base" }
            "#;
            let err = load_registry(src).unwrap_err();
            let expected_err = DimensionError::NotDimensionless {
                name: "dimensionless".into(),
                signature: "base".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_duplicate_name_across_sections() {
            let src = r#"schema = 1

            [registry]
            name = "test-reg"
            version = "0.0.0"

            [base]
            base = { symbol = "B" }

            [dimensionless]
            base = { definition = "base" }
            "#;
            let err = load_registry(src).unwrap_err();
            let expected_err = DimensionError::DuplicateName {
                name: "base".into(),
                registry: "test-reg".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_missing_registry_section() {
            let src = r#"schema = 1

            [base]
            base = { symbol = "B" }
            "#;
            let err = load_registry(src).unwrap_err();
            let expected_err = DimensionError::DefinitionFile(
                "missing [registry] section; a name is required to build a new registry".into(),
            );
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_cyclic_definitions() {
            let src = r#"schema = 1

            [registry]
            name = "test-reg"
            version = "0.0.0"

            [derived]
            a = { definition = "b" }
            b = { definition = "a" }
            "#;
            let err = load_registry(src).unwrap_err();
            let expected_err = DimensionError::CyclicDefinition {
                names: vec!["a".into(), "b".into()],
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_malformed_toml() {
            let bad_toml = r#"schema = 1

            [registry
            name = "test-reg"
            version = "0.0.0"
            "#;
            assert!(matches!(
                load_registry(bad_toml),
                Err(DimensionError::DefinitionFile(_))
            ));
        }

        #[test]
        fn rejects_unknown_referenced_name() {
            let src = r#"schema = 1

            [registry]
            name = "test-reg"
            version = "0.0.0"

            [derived]
            b = { definition = "a" }
            "#;
            let err = load_registry(src).unwrap_err();
            let expected_err = DimensionError::UnknownDimension {
                name: "a".into(),
                registry: "test-reg".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn propagates_duplicate_name_error_from_alias() {
            let src = r#"schema = 1

            [registry]
            name = "test-reg"
            version = "0.0.0"

            [base]
            a = { symbol = "A" }

            [dimensionless]
            b = { definition = "a / a", aliases = ["a"] }
            "#;
            let err = load_registry(src).unwrap_err();
            let expected_err = DimensionError::DuplicateName {
                name: "a".into(),
                registry: "test-reg".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_unknown_field() {
            let src = r#"schema = 1

            [registry]
            name = "test-reg"
            version = "0.0.0"

            [base]
            length = { symbl = "L" }
            "#;
            assert!(matches!(
                load_registry(src),
                Err(DimensionError::DefinitionFile(_))
            ));
        }

        #[test]
        fn rejects_duplicate_key_within_section() {
            let src = r#"schema = 1

            [registry]
            name = "test-reg"
            version = "0.0.0"

            [base]
            length = { symbol = "L" }
            length = { symbol = "M" }
            "#;
            assert!(matches!(
                load_registry(src),
                Err(DimensionError::DefinitionFile(_))
            ));
        }
    }

    mod extend_registry {
        use super::*;
        use crate::registry::DEFAULT_REGISTRY_VERSION;

        #[test]
        fn adds_entries_against_preexisting_names() {
            let mut registry = DimRegistry::new("test-reg");
            registry.add_base("a", Some("A")).unwrap();
            assert!(
                registry.get("b").is_none(),
                "unregistered dimension should not be present in the registry"
            );
            let src = r#"schema = 1

            [derived]
            b = { definition = "a" }
            "#;
            extend_registry(&mut registry, src).unwrap();
            assert!(
                registry.get("b").is_some(),
                "source defined dimension should be in the registry"
            );
        }

        #[test]
        fn ignores_registry_section() {
            let mut registry = DimRegistry::new("test-reg");
            registry.add_base("a", Some("A")).unwrap();
            assert_eq!(registry.name(), "test-reg");
            assert_eq!(registry.version(), DEFAULT_REGISTRY_VERSION);
            let src = r#"schema = 1

            [registry]
            name = "test-reg-override"
            version = "1.0.12"

            [derived]
            b = { definition = "a" }
            "#;
            extend_registry(&mut registry, src).unwrap();
            assert_eq!(
                registry.name(),
                "test-reg",
                "registry name must match before and after extension"
            );
            assert_eq!(
                registry.version(),
                DEFAULT_REGISTRY_VERSION,
                "registry version must match before and after extension"
            )
        }

        #[test]
        fn rejects_duplicate_name_against_existing_registry() {
            let mut registry = DimRegistry::new("test-reg");
            registry.add_base("a", Some("A")).unwrap();
            assert_eq!(registry.name(), "test-reg");
            let src = r#"schema = 1

            [registry]
            name = "test-reg-override"
            version = "0.0.0"

            [base]
            a = { symbol = "B" }
            "#;
            let err = extend_registry(&mut registry, src).unwrap_err();
            let expected_err = DimensionError::DuplicateName {
                name: "a".into(),
                registry: "test-reg".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }
    }
}
