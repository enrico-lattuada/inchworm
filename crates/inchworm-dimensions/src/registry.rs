use std::collections::HashMap;

use crate::{
    AtomId, Dimension, DimensionError, RegistryId,
    atom::{Atom, AtomData, AtomKind},
};

/// A mutable namespace and factory for named dimensions.
///
/// Instance-based: multiple registries coexist. Dimensions from different
/// registries cannot be mixed; the mismatch is detected via the [`RegistryId`]
/// carried by every atom.
///
/// TODO: Add examples
pub struct DimRegistry {
    id: RegistryId,
    name: Box<str>,
    /// Map name to atom.
    atoms: HashMap<Box<str>, Atom>,
}

impl DimRegistry {
    /// Creates an empty registry.
    pub fn new(name: &str) -> Self {
        Self {
            id: RegistryId::next(),
            name: name.into(),
            atoms: HashMap::new(),
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
}

// ---- definition (mutation) ----
impl DimRegistry {
    /// Add a base dimension to the registry and return the corresponding [`Dimension`].
    ///
    /// # Errors
    /// Returns [`DimensionError::DuplicateName`] if `name` is already present in the registry.
    pub fn add_base(&mut self, name: &str, symbol: &str) -> Result<Dimension, DimensionError> {
        // Check name is not duplicated
        if self.atoms.contains_key(name) {
            return Err(DimensionError::DuplicateName {
                name: name.into(),
                registry: self.name().into(),
            });
        }
        let data = AtomData {
            id: AtomId::next(),
            name: name.into(),
            registry_id: self.id(),
            kind: AtomKind::Base {
                symbol: symbol.into(),
            },
        };
        let atom = Atom::new(data);
        let dimension = Dimension::from_atom(&atom);
        self.atoms.insert(name.into(), atom);
        Ok(dimension)
    }

    /// Add a derived dimension to the registry and return the corresponding [`Dimension`].
    ///
    /// # Errors
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
            name: name.into(),
            registry_id: self.id(),
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

    /// Returns the [`Dimension`] corresponding to `name`.
    pub fn get(&self, name: &str) -> Option<Dimension> {
        self.atoms.get(name).map(Dimension::from_atom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod add_base {
        use super::*;
        use crate::{Exp, dimension::Compatibility, test_utils::errors_match};

        #[test]
        fn dimension_has_matching_factors_signature_canonical() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            assert_eq!(length.factors().entries().len(), 1);
            let (_, exp) = length.factors().entries().first().unwrap().to_owned();
            assert_eq!(exp, Exp::ONE);
            assert_eq!(length.factors(), length.canonical_form());
            assert_eq!(length.factors(), &length.signature().0);
        }

        #[test]
        fn rejects_duplicate_name() {
            let mut registry = DimRegistry::new("test_reg");
            registry.add_base("length", "L").unwrap();
            let err = registry.add_base("length", "l").unwrap_err();
            let expected_err = DimensionError::DuplicateName {
                name: String::from("length"),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn distinct_names_are_incompatible() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let time = registry.add_base("time", "T").unwrap();
            assert_ne!(length, time);
            assert!(matches!(
                length.compatibility(&time),
                Compatibility::Incompatible
            ));
        }
    }

    mod add_derived {
        use crate::{Exp, test_utils::errors_match};

        use super::*;

        #[test]
        fn dimensionless_kind_keeps_itself_in_canonical() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let definition = length.try_div(&length).unwrap();
            let strain = registry.add_derived("strain", &definition).unwrap();
            assert!(strain.has_dimensionless_signature());
            assert!(!strain.is_dimensionless());
        }

        #[test]
        fn non_dimensionless_kind_expands_canonical_to_definition() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let time = registry.add_base("time", "T").unwrap();
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
            let length = registry.add_base("length", "L").unwrap();
            let time = registry.add_base("time", "T").unwrap();
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
            let length = registry1.add_base("length", "L").unwrap();
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
            let length = registry.add_base("length", "L").unwrap();
            assert_eq!(registry.get("length").unwrap(), length);
        }
    }
}
