use std::collections::HashMap;

use std::sync::Arc;

use crate::{
    AtomId, Dimension, DimensionError, RegistryId,
    atom::{Atom, AtomData, AtomKind},
    parser::parse_dim_expr,
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
    /// The returned [`Dimension`] is an atom-identified dimension, distinct from the `definition`.
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

    /// Removes `name` from the registry, returning the [`Dimension`] at `name` if `name` was previously in the registry.
    ///
    /// # Errors
    /// Returns [`DimensionError::RemovalBlocked`] if any *registered* definition still references `name`.
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
            if dependents.len() > 0 {
                return Err(DimensionError::RemovalBlocked {
                    name: name.into(),
                    dependents,
                });
            }
        }
        Ok(self
            .atoms
            .remove(name)
            .map(|atom| Dimension::from_atom(&atom)))
    }
}

// ---- parsing and loading ----
impl DimRegistry {
    /// Parses a dimension expression (e.g., "length / time^2") against this registry's names.
    ///
    /// # Errors
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

    mod remove {
        use crate::test_utils::{assert_exactly_eq, errors_match};

        use super::*;

        #[test]
        fn returns_none_for_unregistered_name() {
            let mut registry = DimRegistry::new("test_reg");
            let _ = registry.add_base("length", "L");
            assert!(registry.remove("time").unwrap().is_none());
        }

        #[test]
        fn removes_registered_name_and_returns_its_dimension() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let removed = registry.remove("length").unwrap().unwrap();
            assert_exactly_eq(&length, &removed);
            assert!(registry.get("length").is_none());
        }

        #[test]
        fn unblocked_by_external_dimension_handle() {
            let mut registry = DimRegistry::new("test_reg");
            let _length = registry.add_base("length", "L").unwrap();
            let removed = registry.remove("length");
            assert!(removed.is_ok());
            assert!(registry.get("length").is_none());
        }

        #[test]
        fn blocked_by_direct_dependent() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let time = registry.add_base("time", "T").unwrap();
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
            let length = registry.add_base("length", "L").unwrap();
            let time = registry.add_base("time", "T").unwrap();
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
    }

    mod parse {
        use crate::{Exp, test_utils::errors_match};

        use super::*;

        #[test]
        fn parses_simple_identifier() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let parsed = registry.parse("length").unwrap();
            assert_eq!(length, parsed);
        }

        #[test]
        fn parses_multiplication_with_star() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let time = registry.add_base("time", "T").unwrap();
            let parsed = registry.parse("length * time").unwrap();
            assert_eq!(length.try_mul(&time).unwrap(), parsed);
        }

        #[test]
        fn parses_multiplication_with_cdot() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let time = registry.add_base("time", "T").unwrap();
            let parsed = registry.parse("length · time").unwrap();
            assert_eq!(length.try_mul(&time).unwrap(), parsed);
        }

        #[test]
        fn parses_division() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let time = registry.add_base("time", "T").unwrap();
            let parsed = registry.parse("length / time").unwrap();
            assert_eq!(length.try_div(&time).unwrap(), parsed);
        }

        #[test]
        fn parses_fractional_exponents() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let parsed = registry.parse("length ^ 1/2").unwrap();
            assert_eq!(length.pow(Exp::new(1, 2).unwrap()).unwrap(), parsed);
        }

        #[test]
        fn parses_negative_fractional_exponents() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let parsed = registry.parse("length ^ -1/2").unwrap();
            assert_eq!(length.pow(Exp::new(-1, 2).unwrap()).unwrap(), parsed);
        }

        #[test]
        fn parses_parenthesized_exponents() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let parsed = registry.parse("length ^ (-1/2)").unwrap();
            assert_eq!(length.pow(Exp::new(-1, 2).unwrap()).unwrap(), parsed);
        }

        #[test]
        fn parses_int_exponents() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let parsed = registry.parse("length ^ 3").unwrap();
            assert_eq!(length.pow(Exp::int(3).unwrap()).unwrap(), parsed);
        }

        #[test]
        fn parses_parenthesized_expressions() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let time = registry.add_base("time", "T").unwrap();
            let parsed = registry.parse("(length / time) ^ 2").unwrap();
            assert_eq!(
                length
                    .try_div(&time)
                    .unwrap()
                    .pow(Exp::int(2).unwrap())
                    .unwrap(),
                parsed
            );
        }

        #[test]
        fn parses_parenthesized_expressions_ops() {
            let mut registry = DimRegistry::new("test_reg");
            let length = registry.add_base("length", "L").unwrap();
            let _time = registry.add_base("time", "T").unwrap();
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
        fn rejects_unknown_dimension() {
            let mut registry = DimRegistry::new("test_reg");
            let _length = registry.add_base("length", "L").unwrap();
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
            let _length = registry.add_base("length", "L").unwrap();
            let _time = registry.add_base("time", "T").unwrap();
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
            let _length = registry.add_base("length", "L").unwrap();
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
            let _length = registry.add_base("length", "L").unwrap();
            let err = registry.parse("length ^ 1/").unwrap_err();
            let expected_err = DimensionError::Parse {
                src: "".into(),
                offset: 11,
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
            let _length = registry.add_base("length", "L").unwrap();
            let _time = registry.add_base("time", "T").unwrap();
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
            let _length = registry.add_base("length", "L").unwrap();
            let err = registry.parse("length @").unwrap_err();
            let expected_err = DimensionError::Parse {
                src: "".into(),
                offset: 7,
                message: "".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }
    }
}
