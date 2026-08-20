//! [`DimRegistry`]: the mutable, instance-based namespace dimensions are
//! registered against.
//!
//! Covers definition (`add_base`/`add_derived`/`add_dimensionless`/`add_alias`),
//! removal, lookup (`get`/`parse`), and (behind the `toml` feature)
//! TOML-backed loading and construction.

use std::collections::HashMap;

use std::sync::Arc;

use crate::{
    AtomId, Dimension, DimensionError, RegistryId,
    atom::{Atom, AtomData, AtomKind},
    parser::parse_dim_expr,
};

#[cfg(feature = "toml")]
use crate::loader::{extend_registry, load_registry};

pub(crate) const DEFAULT_REGISTRY_VERSION: &str = "0.0.0";

/// A mutable namespace and factory for named dimensions.
///
/// Instance-based: multiple registries coexist. Dimensions from different
/// registries cannot be mixed; the mismatch is detected via the [`RegistryId`]
/// carried by every atom.
///
/// # Examples
///
/// ```
/// use inchworm_dimensions::DimRegistry;
///
/// let mut registry = DimRegistry::new("mechanics");
/// registry.add_base("length", Some("L")).unwrap();
/// registry.add_base("time", Some("T")).unwrap();
/// let velocity = registry.add_derived_expr("velocity", "length / time").unwrap();
///
/// // `get` and `parse` agree for anything already registered.
/// assert_eq!(registry.get("velocity"), Some(velocity.clone()));
/// assert_eq!(registry.parse("length / time").unwrap(), velocity);
///
/// // Removal is blocked while something still depends on a name.
/// assert!(registry.remove("length").is_err());
/// registry.remove("velocity").unwrap();
/// assert!(registry.remove("length").is_ok());
/// ```
#[cfg_attr(test, derive(Debug))]
pub struct DimRegistry {
    id: RegistryId,
    name: Box<str>,
    version: Box<str>,
    /// Map name to atom.
    atoms: HashMap<Box<str>, Atom>,
    aliases: HashMap<Box<str>, Box<str>>,
}

impl DimRegistry {
    /// Creates an empty registry.
    pub fn new(name: &str) -> Self {
        Self::new_with_meta(name, DEFAULT_REGISTRY_VERSION)
    }

    /// Creates an empty registry with a name and a version.
    pub(crate) fn new_with_meta(name: &str, version: &str) -> Self {
        Self {
            id: RegistryId::next(),
            name: name.into(),
            version: version.into(),
            atoms: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// Returns the `id` of the registry.
    pub fn id(&self) -> RegistryId {
        self.id
    }

    /// Returns the `name` of the registry.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the `version` of the registry.
    pub fn version(&self) -> &str {
        &self.version
    }
}

// ---- definition (mutation) ----
impl DimRegistry {
    /// Add a base dimension to the registry and return the corresponding [`Dimension`].
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::DuplicateName`] if `name` is already present in the registry.
    pub fn add_base(
        &mut self,
        name: &str,
        symbol: Option<&str>,
    ) -> Result<Dimension, DimensionError> {
        // Check name is not duplicated
        if self.atoms.contains_key(name) {
            return Err(DimensionError::DuplicateName {
                name: name.into(),
                registry: self.name().into(),
            });
        }
        let data = AtomData {
            id: AtomId::next(),
            registry_id: self.id(),
            name: name.into(),
            symbol: symbol.map(Into::into),
            kind: AtomKind::Base,
        };
        let atom = Atom::new(data);
        let dimension = Dimension::from_atom(&atom);
        self.atoms.insert(name.into(), atom);
        Ok(dimension)
    }

    /// Add a derived dimension to the registry and return the corresponding [`Dimension`].
    ///
    /// The returned [`Dimension`] is an atom-identified dimension, distinct from the `definition`.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::DuplicateName`] if `name` is already present in the registry.
    /// Returns [`DimensionError::CrossRegistry`] if `definition` comes from a different registry.
    pub fn add_derived(
        &mut self,
        name: &str,
        definition: &Dimension,
    ) -> Result<Dimension, DimensionError> {
        if self.atoms.contains_key(name) {
            return Err(DimensionError::DuplicateName {
                name: name.into(),
                registry: self.name().into(),
            });
        }
        if let Some(def_id) = definition.registry_id()
            && self.id() != def_id
        {
            return Err(DimensionError::CrossRegistry {
                left: self.id(),
                right: def_id,
            });
        }
        let data = AtomData {
            id: AtomId::next(),
            registry_id: self.id(),
            name: name.into(),
            symbol: None,
            kind: AtomKind::Derived {
                definition: Box::new(definition.clone()),
                dimensionless_kind: definition.is_dimensionless(),
            },
        };
        let atom = Atom::new(data);
        let dimension = Dimension::from_atom(&atom);
        self.atoms.insert(name.into(), atom);
        Ok(dimension)
    }

    /// Add a derived dimension to the registry via literal expression and return the corresponding [`Dimension`].
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::UnknownDimension`] if `expr` contains a dimension unknown to the registry.
    /// Returns [`DimensionError::Parse`] if `expr` cannot be correctly parsed.
    /// Returns [`DimensionError::DuplicateName`] if `name` is already present in the registry.
    pub fn add_derived_expr(
        &mut self,
        name: &str,
        expr: &str,
    ) -> Result<Dimension, DimensionError> {
        let definition = self.parse(expr)?;
        self.add_derived(name, &definition)
    }

    /// Add a dimensionless, derived dimension to the registry and return the corresponding [`Dimension`].
    ///
    /// The returned [`Dimension`] is an atom-identified dimension, distinct from the `definition`.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::NotDimensionless`] if `definition` does not have a dimensionless signature.
    /// Returns [`DimensionError::DuplicateName`] if `name` is already present in the registry.
    /// Returns [`DimensionError::CrossRegistry`] if `definition` comes from a different registry.
    pub fn add_dimensionless(
        &mut self,
        name: &str,
        definition: &Dimension,
    ) -> Result<Dimension, DimensionError> {
        if !definition.has_dimensionless_signature() {
            Err(DimensionError::NotDimensionless {
                name: name.into(),
                signature: definition.signature().to_string(),
            })
        } else {
            self.add_derived(name, definition)
        }
    }

    /// Add a dimensionless, derived dimension to the registry via literal expression and return the corresponding [`Dimension`].
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::UnknownDimension`] if `expr` contains a dimension unknown to the registry.
    /// Returns [`DimensionError::Parse`] if `expr` cannot be correctly parsed.
    /// Returns [`DimensionError::NotDimensionless`] if `definition` does not have a dimensionless signature.
    /// Returns [`DimensionError::DuplicateName`] if `name` is already present in the registry.
    pub fn add_dimensionless_expr(
        &mut self,
        name: &str,
        expr: &str,
    ) -> Result<Dimension, DimensionError> {
        let definition = self.parse(expr)?;
        self.add_dimensionless(name, &definition)
    }

    /// Registers `alias` as an alternate name for the dimension already registered under `target`.
    ///
    /// `target` may itself be an existing alias; it resolves to the canonical name it ultimately
    /// points to, so aliases never chain.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::DuplicateName`] if `alias` is already taken, either as a canonical
    /// name or as an existing alias.
    /// Returns [`DimensionError::UnknownDimension`] if `target` is not a registered canonical name
    /// or alias.
    pub fn add_alias(&mut self, alias: &str, target: &str) -> Result<(), DimensionError> {
        if self.atoms.contains_key(alias) || self.aliases.contains_key(alias) {
            return Err(DimensionError::DuplicateName {
                name: alias.into(),
                registry: self.name().into(),
            });
        }
        let canonical = if self.atoms.contains_key(target) {
            target.into()
        } else {
            self.aliases
                .get(target)
                .cloned()
                .ok_or_else(|| DimensionError::UnknownDimension {
                    name: target.into(),
                    registry: self.name().into(),
                })?
        };
        self.aliases.insert(alias.into(), canonical);
        Ok(())
    }

    /// Returns the [`Dimension`] corresponding to `name`.
    pub fn get(&self, name: &str) -> Option<Dimension> {
        self.atoms
            .get(name)
            .or_else(|| {
                self.aliases
                    .get(name)
                    .and_then(|canonical| self.atoms.get(canonical))
            })
            .map(Dimension::from_atom)
    }

    /// Removes `name` from the registry, returning the [`Dimension`] at `name` if `name` was a
    /// registered canonical name.
    ///
    /// If `name` is canonical, removing it also removes every alias that pointed to it. If `name`
    /// is itself only an alias, only that alias is removed: the canonical dimension and any of its
    /// other aliases are left untouched, and `Ok(None)` is returned, since an alias never has a
    /// [`Dimension`] identity of its own to hand back.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::RemovalBlocked`] if any *registered* definition still references
    /// `name`. Aliases are never counted as dependents and never block removal.
    pub fn remove(&mut self, name: &str) -> Result<Option<Dimension>, DimensionError> {
        if let Some(atom) = self.atoms.get(name)
            && Arc::strong_count(atom) > 1
        {
            let mut dependents = Vec::new();
            for (atom_name, atom_data) in self.atoms.iter() {
                let AtomKind::Derived { definition, .. } = &atom_data.kind else {
                    continue;
                };
                let factors_entries = definition.factors().entries();
                let canonical_entries = definition.canonical_form().entries();
                if factors_entries
                    .iter()
                    .chain(canonical_entries.iter())
                    .any(|(other_atom, _)| Arc::ptr_eq(atom, other_atom))
                {
                    dependents.push(atom_name.clone().into());
                }
            }
            if !dependents.is_empty() {
                return Err(DimensionError::RemovalBlocked {
                    name: name.into(),
                    dependents,
                });
            }
        }
        let dimension = self
            .atoms
            .remove(name)
            .map(|atom| Dimension::from_atom(&atom));
        match &dimension {
            None => {
                self.aliases.remove(name);
                return Ok(None);
            }
            Some(_) => {
                self.aliases
                    .retain(|_, canonical| canonical.as_ref() != name);
            }
        }
        Ok(dimension)
    }
}

// ---- parsing and loading ----
impl DimRegistry {
    /// Parses a dimension expression (e.g., "length / time^2") against this registry's names.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::UnknownDimension`] if `expr` contains a dimension unknown to the registry.
    /// Returns [`DimensionError::Parse`] if `expr` cannot be correctly parsed.
    pub fn parse(&self, expr: &str) -> Result<Dimension, DimensionError> {
        parse_dim_expr(expr, &|name| {
            self.get(name)
                .ok_or_else(|| DimensionError::UnknownDimension {
                    name: name.into(),
                    registry: self.name().into(),
                })
        })
    }

    /// Extends the registry with the source definitions in TOML format.
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
    #[cfg(feature = "toml")]
    pub fn load_toml_str(&mut self, src: &str) -> Result<(), DimensionError> {
        extend_registry(self, src)
    }

    /// Extends the registry with the source definitions from the TOML file at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::DefinitionFile`] if the TOML file cannot be read or deserialized correctly.
    /// Returns [`DimensionError::UnknownDimension`] if any dimension referenced by the source was never defined.
    /// Returns [`DimensionError::DuplicateName`] if duplicate names are found in the source.
    /// Returns [`DimensionError::CyclicDefinition`] if cyclic definitions are found in the source.
    /// Returns [`DimensionError::NotDimensionless`] if a declared dimensionless dimension is not in fact dimensionless.
    /// Returns [`DimensionError::Parse`] if the information in the source cannot be parsed.
    /// Returns [`DimensionError::ExponentOverflow`] if any exponents in the source definitions incur overflow.
    /// Returns [`DimensionError::ZeroDenominator`] if any exponents' denominators in the source definitions are zero.
    #[cfg(feature = "toml")]
    pub fn load_toml_file(&mut self, path: &std::path::Path) -> Result<(), DimensionError> {
        let src = std::fs::read_to_string(path)
            .map_err(|e| DimensionError::DefinitionFile(e.to_string()))?;
        self.load_toml_str(&src)
    }

    /// Creates a new [`DimRegistry`] from the source definitions in TOML format.
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
    #[cfg(feature = "toml")]
    pub fn from_toml_str(src: &str) -> Result<Self, DimensionError> {
        load_registry(src)
    }

    /// Creates a new [`DimRegistry`] with the source definitions from the TOML file at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::DefinitionFile`] if the TOML file cannot be read or deserialized correctly.
    /// Returns [`DimensionError::UnknownDimension`] if any dimension referenced by the source was never defined.
    /// Returns [`DimensionError::DuplicateName`] if duplicate names are found in the source.
    /// Returns [`DimensionError::CyclicDefinition`] if cyclic definitions are found in the source.
    /// Returns [`DimensionError::NotDimensionless`] if a declared dimensionless dimension is not in fact dimensionless.
    /// Returns [`DimensionError::Parse`] if the information in the source cannot be parsed.
    /// Returns [`DimensionError::ExponentOverflow`] if any exponents in the source definitions incur overflow.
    /// Returns [`DimensionError::ZeroDenominator`] if any exponents' denominators in the source definitions are zero.
    #[cfg(feature = "toml")]
    pub fn from_toml_file(path: &std::path::Path) -> Result<Self, DimensionError> {
        let src = std::fs::read_to_string(path)
            .map_err(|e| DimensionError::DefinitionFile(e.to_string()))?;
        load_registry(&src)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assert_exactly_eq, errors_match};

    mod add_base {
        use super::*;
        use crate::{Exp, dimension::Compatibility};

        #[test]
        fn dimension_has_matching_factors_signature_canonical() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            assert_eq!(length.factors().entries().len(), 1);
            let (_, exp) = length.factors().entries().first().unwrap().to_owned();
            assert_eq!(exp, Exp::ONE);
            assert_eq!(length.factors(), length.canonical_form());
            assert_eq!(length.factors(), &length.signature().0);
        }

        #[test]
        fn rejects_duplicate_name() {
            let mut registry = DimRegistry::new("test_reg");
            registry.add_base("length", Some("L")).unwrap();
            let err = registry.add_base("length", Some("l")).unwrap_err();
            let expected_err = DimensionError::DuplicateName {
                name: String::from("length"),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn distinct_names_are_incompatible() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            assert_ne!(length, time);
            assert!(matches!(
                length.compatibility(&time),
                Compatibility::Incompatible
            ));
        }
    }

    mod add_derived {
        use super::*;
        use crate::Exp;

        #[test]
        fn dimensionless_kind_keeps_itself_in_canonical() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let definition = length.try_div(&length).unwrap();
            let strain = registry.add_derived("strain", &definition).unwrap();
            assert!(strain.has_dimensionless_signature());
            assert!(!strain.is_dimensionless());
        }

        #[test]
        fn non_dimensionless_kind_expands_canonical_to_definition() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            let definition = length.try_div(&time).unwrap();
            let velocity = registry.add_derived("velocity", &definition).unwrap();
            assert!(!velocity.is_dimensionless());
            assert!(!velocity.has_dimensionless_signature());
            assert_eq!(velocity.factors().entries().len(), 1);
            let (atom, exp) = velocity.factors().entries().first().unwrap().to_owned();
            assert_eq!(atom.name, "velocity".into());
            assert_eq!(exp, Exp::ONE);
            assert_eq!(velocity, definition);
            assert_ne!(velocity.factors(), definition.factors());
        }

        #[test]
        fn rejects_duplicate_name() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            let definition = length.try_div(&time).unwrap();
            registry.add_derived("velocity", &definition).unwrap();
            let err = registry
                .add_derived("velocity", &definition.clone())
                .unwrap_err();
            let expected_err = DimensionError::DuplicateName {
                name: String::from("velocity"),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_definition_from_different_registry() {
            let mut registry1 = DimRegistry::new("test_reg_1");
            let length = registry1.add_base("length", Some("L")).unwrap();
            let mut registry2 = DimRegistry::new("test_reg_2");
            let err = registry2.add_derived("length", &length).unwrap_err();
            let expected_err = DimensionError::CrossRegistry {
                left: registry2.id(),
                right: registry1.id(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn allows_dimensionless_definition_from_any_registry() {
            let mut registry1 = DimRegistry::new("test_reg_1");
            let mut registry2 = DimRegistry::new("test_reg_2");
            assert!(
                registry1
                    .add_derived("count", &Dimension::dimensionless())
                    .is_ok()
            );
            assert!(
                registry2
                    .add_derived("count", &Dimension::dimensionless())
                    .is_ok()
            );
        }
    }

    mod add_derived_expr {
        use super::*;

        #[test]
        fn adds_dimension_from_expression() {
            let mut registry = DimRegistry::new("test-reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            let velocity = registry
                .add_derived_expr("velocity", "length / time")
                .unwrap();
            assert_eq!(velocity, length.try_div(&time).unwrap());
        }

        #[test]
        fn propagates_parse_error() {
            let mut registry = DimRegistry::new("test-reg");
            assert!(registry.get("nonexistent").is_none());
            let err = registry.add_derived_expr("foo", "nonexistent").unwrap_err();
            let expected_err = DimensionError::UnknownDimension {
                name: "nonexistent".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn propagates_duplicate_name_error() {
            let mut registry = DimRegistry::new("test-reg");
            registry.add_base("length", Some("L")).unwrap();
            let err = registry.add_derived_expr("length", "1").unwrap_err();
            let expected_err = DimensionError::DuplicateName {
                name: "length".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod add_dimensionless {
        use super::*;

        #[test]
        fn allows_dimensionless_definition() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let definition = length.try_div(&length).unwrap();
            let plane_angle = registry
                .add_dimensionless("plane_angle", &definition)
                .unwrap();
            assert!(plane_angle.has_dimensionless_signature());
            assert!(!plane_angle.is_dimensionless());
        }

        #[test]
        fn rejects_non_dimensionless_signature_definition() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let err = registry.add_dimensionless("distance", &length).unwrap_err();
            let expected_err = DimensionError::NotDimensionless {
                name: "distance".into(),
                signature: "length".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod add_dimensionless_expr {
        use super::*;

        #[test]
        fn add_dimensionless_dimension_from_expr() {
            let mut registry = DimRegistry::new("test_reg");
            registry.add_base("length", Some("L")).unwrap();
            let plane_angle = registry
                .add_dimensionless_expr("plane_angle", "length / length")
                .unwrap();
            assert!(plane_angle.has_dimensionless_signature());
            assert!(!plane_angle.is_dimensionless());
        }

        #[test]
        fn propagates_not_dimensionless_error() {
            let mut registry = DimRegistry::new("test_reg");
            registry.add_base("length", Some("L")).unwrap();
            let err = registry
                .add_dimensionless_expr("distance", "length")
                .unwrap_err();
            let expected_err = DimensionError::NotDimensionless {
                name: "distance".into(),
                signature: "length".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn propagates_parse_error() {
            let mut registry = DimRegistry::new("test-reg");
            assert!(registry.get("nonexistent").is_none());
            let err = registry
                .add_dimensionless_expr("foo", "nonexistent / nonexistent")
                .unwrap_err();
            let expected_err = DimensionError::UnknownDimension {
                name: "nonexistent".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod add_alias {
        use super::*;

        #[test]
        fn aliasing_an_alias_resolves_to_the_original_canonical_name() {
            let mut registry = DimRegistry::new("test_reg");
            let a = registry.add_base("a", Some("A")).unwrap();
            registry.add_alias("b", "a").unwrap();
            registry.add_alias("c", "b").unwrap();
            assert_exactly_eq(&registry.get("c").unwrap(), &a);
            registry.remove("b").unwrap();
            assert_exactly_eq(&registry.get("c").unwrap(), &a);
        }

        #[test]
        fn rejects_alias_that_shadows_existing_canonical_name() {
            let mut registry = DimRegistry::new("test_reg");
            registry.add_base("a", Some("A")).unwrap();
            registry.add_base("b", Some("B")).unwrap();
            let err = registry.add_alias("a", "b").unwrap_err();
            let expected_err = DimensionError::DuplicateName {
                name: "a".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_alias_that_shadows_existing_alias() {
            let mut registry = DimRegistry::new("test_reg");
            registry.add_base("a", Some("A")).unwrap();
            registry.add_base("b", Some("B")).unwrap();
            registry.add_alias("c", "a").unwrap();
            let err = registry.add_alias("c", "b").unwrap_err();
            let expected_err = DimensionError::DuplicateName {
                name: "c".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_unknown_canonical_target() {
            let mut registry = DimRegistry::new("test_reg");
            let err = registry.add_alias("b", "a").unwrap_err();
            let expected_err = DimensionError::UnknownDimension {
                name: "a".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod get {
        use super::*;

        #[test]
        fn returns_none_for_unregistered_name() {
            let registry = DimRegistry::new("test_reg");
            assert!(registry.get("non-existent").is_none());
        }

        #[test]
        fn matches_what_add_base_returned() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            assert_eq!(registry.get("length").unwrap(), length);
        }

        #[test]
        fn resolves_alias_to_same_dimension_as_canonical_name() {
            let mut registry = DimRegistry::new("test_reg");
            let a = registry.add_base("a", Some("A")).unwrap();
            registry.add_alias("b", "a").unwrap();
            assert_exactly_eq(&registry.get("b").unwrap(), &a);
        }
    }

    mod remove {
        use super::*;

        #[test]
        fn returns_none_for_unregistered_name() {
            let mut registry = DimRegistry::new("test_reg");
            let _ = registry.add_base("length", Some("L"));
            assert!(registry.remove("time").unwrap().is_none());
        }

        #[test]
        fn removes_registered_name_and_returns_its_dimension() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let removed = registry.remove("length").unwrap().unwrap();
            assert_exactly_eq(&length, &removed);
            assert!(registry.get("length").is_none());
        }

        #[test]
        fn unblocked_by_external_dimension_handle() {
            let mut registry = DimRegistry::new("test_reg");
            let _length = registry.add_base("length", Some("L")).unwrap();
            let removed = registry.remove("length");
            assert!(removed.is_ok());
            assert!(registry.get("length").is_none());
        }

        #[test]
        fn blocked_by_direct_dependent() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            let definition = length.try_div(&time).unwrap();
            let _velocity = registry.add_derived("velocity", &definition).unwrap();
            let err = registry.remove("length").unwrap_err();
            let expected_err = DimensionError::RemovalBlocked {
                name: "length".into(),
                dependents: vec!["velocity".into()],
            };
            assert!(errors_match(&err, &expected_err,));
            assert!(registry.get("length").is_some());
        }

        #[test]
        fn blocked_by_transitive_canonical_dependent() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            let velocity_definition = length.try_div(&time).unwrap();
            let velocity = registry
                .add_derived("velocity", &velocity_definition)
                .unwrap();
            let acceleration_definition = velocity.try_div(&time).unwrap();
            let _acceleration = registry
                .add_derived("acceleration", &acceleration_definition)
                .unwrap();
            let err = registry.remove("length").unwrap_err();
            let expected_err = DimensionError::RemovalBlocked {
                name: "length".into(),
                dependents: vec!["velocity".into(), "acceleration".into()],
            };
            assert!(errors_match(&err, &expected_err,));
            assert!(registry.get("length").is_some());
        }

        #[test]
        fn removing_canonical_name_cascades_removal_of_its_aliases() {
            let mut registry = DimRegistry::new("test_reg");
            registry.add_base("a", Some("A")).unwrap();
            registry.add_alias("b", "a").unwrap();
            registry.add_alias("c", "a").unwrap();
            assert!(registry.get("b").is_some() && registry.get("c").is_some());
            registry.remove("a").unwrap();
            assert!(registry.get("b").is_none() && registry.get("c").is_none());
        }

        #[test]
        fn removing_alias_leaves_canonical_and_other_aliases_untouched() {
            let mut registry = DimRegistry::new("test_reg");
            registry.add_base("a", Some("A")).unwrap();
            registry.add_alias("b", "a").unwrap();
            registry.add_alias("c", "a").unwrap();
            registry.remove("b").unwrap();
            assert!(registry.get("a").is_some());
            assert!(registry.get("b").is_none());
            assert!(registry.get("c").is_some());
        }
    }

    mod parse {
        use super::*;
        use crate::Exp;

        #[test]
        fn parses_simple_identifier() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let parsed = registry.parse("length").unwrap();
            assert_eq!(length, parsed);
        }

        #[test]
        fn parses_multiplication_with_star() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            let parsed = registry.parse("length * time").unwrap();
            assert_eq!(length * time, parsed);
        }

        #[test]
        fn parses_multiplication_with_cdot() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            let parsed = registry.parse("length · time").unwrap();
            assert_eq!(length * time, parsed);
        }

        #[test]
        fn parses_division() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            let parsed = registry.parse("length / time").unwrap();
            assert_eq!(length / time, parsed);
        }

        #[test]
        fn parses_fractional_exponents() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let parsed = registry.parse("length ^ (1/2)").unwrap();
            assert_eq!(length.pow(Exp::new(1, 2).unwrap()).unwrap(), parsed);
        }

        #[test]
        fn parses_negative_fractional_exponents() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let parsed = registry.parse("length ^ (-1/2)").unwrap();
            assert_eq!(length.pow(Exp::new(-1, 2).unwrap()).unwrap(), parsed);
        }

        #[test]
        fn parses_outer_negative_fractional_exponents() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let parsed = registry.parse("length ^ -(1/2)").unwrap();
            assert_eq!(length.pow(Exp::new(-1, 2).unwrap()).unwrap(), parsed);
        }

        #[test]
        fn parses_double_negative_fractional_exponents() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let parsed = registry.parse("length ^ -(-1/2)").unwrap();
            assert_eq!(length.pow(Exp::new(1, 2).unwrap()).unwrap(), parsed);
        }

        #[test]
        fn parses_int_exponents() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let parsed = registry.parse("length ^ 3").unwrap();
            assert_eq!(length.pow(Exp::int(3)).unwrap(), parsed);
        }

        #[test]
        fn parses_parenthesized_int_exponents() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let parsed = registry.parse("length ^ (3)").unwrap();
            assert_eq!(length.pow(Exp::int(3)).unwrap(), parsed);
        }

        #[test]
        fn parses_parenthesized_expressions() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            let parsed = registry.parse("(length / time) ^ 2").unwrap();
            assert_eq!(
                length.try_div(&time).unwrap().pow(Exp::int(2)).unwrap(),
                parsed
            );
        }

        #[test]
        fn parses_parenthesized_expressions_ops() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let _time = registry.add_base("time", Some("T")).unwrap();
            let parsed = registry.parse("(length / time) * (time)").unwrap();
            assert_eq!(length, parsed);
        }

        #[test]
        fn parses_bare_one_as_dimensionless() {
            let registry = DimRegistry::new("test_reg");
            let parsed = registry.parse("1").unwrap();
            assert_eq!(Dimension::dimensionless(), parsed);
        }

        #[test]
        fn parses_bare_integer_exponent_followed_by_division() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let parsed = registry.parse("length^2 / length^2").unwrap();
            let length_squared = length.pow(Exp::int(2)).unwrap();
            assert_eq!(&length_squared / &length_squared, parsed);
        }

        #[test]
        fn rejects_unknown_dimension() {
            let mut registry = DimRegistry::new("test_reg");
            let _length = registry.add_base("length", Some("L")).unwrap();
            let err = registry.parse("time").unwrap_err();
            let expected_err = DimensionError::UnknownDimension {
                name: "time".into(),
                registry: "test_reg".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_trailing_garbage() {
            let mut registry = DimRegistry::new("test_reg");
            let _length = registry.add_base("length", Some("L")).unwrap();
            let _time = registry.add_base("time", Some("T")).unwrap();
            let err = registry.parse("length time").unwrap_err();
            let expected_err = DimensionError::Parse {
                src: "".into(),
                offset: 7,
                message: "".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_unmatched_parenthesis() {
            let mut registry = DimRegistry::new("test_reg");
            let _length = registry.add_base("length", Some("L")).unwrap();
            let err = registry.parse("(length").unwrap_err();
            let expected_err = DimensionError::Parse {
                src: "".into(),
                offset: 7,
                message: "".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_invalid_exponent_fraction() {
            let mut registry = DimRegistry::new("test_reg");
            let _length = registry.add_base("length", Some("L")).unwrap();
            let err = registry.parse("length ^ (1/)").unwrap_err();
            let expected_err = DimensionError::Parse {
                src: "".into(),
                offset: 12,
                message: "".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_bare_fractional_exponent() {
            let mut registry = DimRegistry::new("test_reg");
            registry.add_base("length", Some("L")).unwrap();
            let err = registry.parse("length ^ 1/2").unwrap_err();
            let expected_err = DimensionError::Parse {
                src: "".into(),
                offset: 11,
                message: "".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_bare_negative_fractional_exponent() {
            let mut registry = DimRegistry::new("test_reg");
            registry.add_base("length", Some("L")).unwrap();
            let err = registry.parse("length ^ -1/2").unwrap_err();
            let expected_err = DimensionError::Parse {
                src: "".into(),
                offset: 12,
                message: "".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_non_one_number_as_factor() {
            let registry = DimRegistry::new("test_reg");
            let err = registry.parse("2").unwrap_err();
            let expected_err = DimensionError::Parse {
                src: "".into(),
                offset: 0,
                message: "".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_empty_expr() {
            let registry = DimRegistry::new("test_reg");
            let err = registry.parse("").unwrap_err();
            let expected_err = DimensionError::Parse {
                src: "".into(),
                offset: 0,
                message: "".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_invalid_pow() {
            let mut registry = DimRegistry::new("test_reg");
            let _length = registry.add_base("length", Some("L")).unwrap();
            let _time = registry.add_base("time", Some("T")).unwrap();
            let err = registry.parse("length ^ time").unwrap_err();
            let expected_err = DimensionError::Parse {
                src: "".into(),
                offset: 9,
                message: "".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_unsupported_character() {
            let mut registry = DimRegistry::new("test_reg");
            let _length = registry.add_base("length", Some("L")).unwrap();
            let err = registry.parse("length @").unwrap_err();
            let expected_err = DimensionError::Parse {
                src: "".into(),
                offset: 7,
                message: "".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }
    }

    #[cfg(feature = "toml")]
    mod load_toml_str {
        use super::*;

        #[test]
        fn extends_registry_with_valid_source() {
            let mut registry = DimRegistry::new("test_reg");
            assert!(registry.get("base").is_none());
            let src = r#"schema = 1

            [[base]]
            name = "base"
            symbol = "B"
            "#;
            registry.load_toml_str(src).unwrap();
            assert!(
                registry.get("base").is_some(),
                "source defined dimension should be in the registry"
            );
        }

        #[test]
        fn propagates_error_from_source() {
            let mut registry = DimRegistry::new("test-reg");
            let bad_toml = r#"schema = 1

            [registry
            name = "test-reg"
            version = "0.0.0"
            "#;
            assert!(matches!(
                registry.load_toml_str(bad_toml),
                Err(DimensionError::DefinitionFile(_))
            ));
        }
    }

    #[cfg(feature = "toml")]
    mod load_toml_file {
        use super::*;
        use std::io::Write;

        #[test]
        fn loads_from_existing_file() {
            let mut file = tempfile::NamedTempFile::new().unwrap();
            file.write_all(
                br#"schema = 1

                [[base]]
                name = "base"
                symbol = "B"
                "#,
            )
            .unwrap();
            let mut registry = DimRegistry::new("test-reg");
            registry.load_toml_file(file.path()).unwrap();
            assert!(registry.get("base").is_some());
        }

        #[test]
        fn propagates_io_error_for_missing_file() {
            let bad_path = std::env::temp_dir().join("nonexistent.toml");
            let mut registry = DimRegistry::new("test-reg");
            assert!(matches!(
                registry.load_toml_file(bad_path.as_path()),
                Err(DimensionError::DefinitionFile(_))
            ))
        }
    }

    #[cfg(feature = "toml")]
    mod from_toml_str {
        use super::*;

        #[test]
        fn builds_registry_with_valid_source() {
            let src = r#"schema = 1

            [registry]
            name = "test-registry"
            version = "0.1.0"

            [[base]]
            name = "base"
            symbol = "B"
            "#;
            let registry = DimRegistry::from_toml_str(src).unwrap();
            assert_eq!(registry.name(), "test-registry");
            assert!(
                registry.get("base").is_some(),
                "source defined dimension should be in the registry"
            );
        }

        #[test]
        fn propagates_error_from_source() {
            let bad_toml = r#"schema = 1

            [registry
            name = "test-reg"
            version = "0.0.0"
            "#;
            assert!(matches!(
                DimRegistry::from_toml_str(bad_toml),
                Err(DimensionError::DefinitionFile(_))
            ));
        }
    }

    #[cfg(feature = "toml")]
    mod from_toml_file {
        use super::*;
        use std::io::Write;

        #[test]
        fn builds_registry_from_existing_file() {
            let mut file = tempfile::NamedTempFile::new().unwrap();
            file.write_all(
                br#"schema = 1

                [registry]
                name = "test-registry"
                version = "0.1.0"

                [[base]]
                name = "base"
                symbol = "B"
                "#,
            )
            .unwrap();
            let registry = DimRegistry::from_toml_file(file.path()).unwrap();
            assert_eq!(registry.name(), "test-registry");
            assert!(registry.get("base").is_some());
        }

        #[test]
        fn propagates_io_error_for_missing_file() {
            let bad_path = std::env::temp_dir().join("nonexistent.toml");
            assert!(matches!(
                DimRegistry::from_toml_file(bad_path.as_path()),
                Err(DimensionError::DefinitionFile(_))
            ))
        }
    }

    mod roundtrips {
        use super::*;
        use crate::Exp;

        #[test]
        fn roundtrips_through_display_and_parse() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let time = registry.add_base("time", Some("T")).unwrap();
            let mass = registry.add_base("mass", Some("M")).unwrap();
            let definitions = [
                &length / &time.pow(Exp::int(2)).unwrap(),
                length.pow(Exp::new(3, 2).unwrap()).unwrap(),
                &length * &time * &mass,
            ];
            for (i, definition) in definitions.iter().enumerate() {
                let name = format!("{i}");
                registry.add_derived(&name, definition).unwrap();
                let str_definition = definition.factors().to_string();
                let parsed = registry.parse(&str_definition).unwrap();
                assert_exactly_eq(&parsed, definition);
            }
        }
    }
}
