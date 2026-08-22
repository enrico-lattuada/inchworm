//! [`UnitRegistry`]: the mutable, instance-based namespace units are
//! registered against.

use std::{collections::HashMap, sync::Arc};

use inchworm_dimensions::{DimRegistry, Dimension, Exp};

use crate::{
    Unit, UnitError, UnitId, UnitRegistryId,
    atom::{ConversionKind, UnitData},
};

pub(crate) const DEFAULT_REGISTRY_VERSION: &str = "0";

/// A mutable namespace and factory for named units.
///
/// Instance-based: multiple registries coexist. Units from different
/// registries cannot be mixed; the mismatch is detected via the [`UnitRegistryId`]
/// carried by every atom.
pub struct UnitRegistry {
    id: UnitRegistryId,
    dims: DimRegistry,
    name: Box<str>,
    version: Box<str>,
    /// Map name to atom.
    atoms: HashMap<Box<str>, Arc<UnitData>>,
}

impl UnitRegistry {
    /// Creates an empty registry associated with `dims` with a name and a default version.
    pub fn new(name: &str, dims: DimRegistry) -> Self {
        Self::new_with_meta(name, DEFAULT_REGISTRY_VERSION, dims)
    }

    /// Creates an empty registry associated with `dims` with a name and a version.
    pub(crate) fn new_with_meta(name: &str, version: &str, dims: DimRegistry) -> Self {
        Self {
            id: UnitRegistryId::next(),
            dims,
            name: name.into(),
            version: version.into(),
            atoms: HashMap::new(),
        }
    }

    /// Returns the `id` of the registry.
    pub fn id(&self) -> UnitRegistryId {
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

    /// Returns the `dims` of the registry.
    pub fn dims(&self) -> &DimRegistry {
        &self.dims
    }
}

// ---- definition (mutation) ----
impl UnitRegistry {
    /// Add a unit to the registry and return the corresponding [`Unit`].
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::DuplicateName`] if `name` is already present in the registry.
    /// Propagates [`UnitError::Dimension`] from the underlying dimension algebra.
    pub fn add_unit(
        &mut self,
        name: &str,
        symbol: &str,
        dimension: Dimension,
        scale: f64,
    ) -> Result<Unit, UnitError> {
        if self.atoms.contains_key(name) {
            return Err(UnitError::DuplicateName {
                name: name.into(),
                registry: self.name().into(),
            });
        }
        let data = UnitData {
            id: UnitId::next(),
            registry_id: self.id(),
            name: name.into(),
            symbol: symbol.into(),
            dimension,
            conversion: ConversionKind::Linear { scale },
        };
        let atom = Arc::new(data);
        let unit = Unit::single(&atom, Exp::ONE)?;
        self.atoms.insert(name.into(), atom);
        Ok(unit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod new {
        use super::*;

        #[test]
        fn assigns_unique_ids() {
            let dims_a = DimRegistry::new("test-reg-a");
            let registry_a = UnitRegistry::new("test-ureg-a", dims_a);
            let dims_b = DimRegistry::new("test-reg-b");
            let registry_b = UnitRegistry::new("test-ureg-b", dims_b);
            assert_ne!(registry_a.id(), registry_b.id());
        }

        #[test]
        fn stores_name_and_default_version() {
            let dims = DimRegistry::new("test-reg");
            let registry = UnitRegistry::new("test-ureg", dims);
            assert_eq!(registry.name(), "test-ureg");
            assert_eq!(registry.version(), DEFAULT_REGISTRY_VERSION);
        }

        #[test]
        fn stores_dims() {
            let dims_name = "test_reg";
            let dims = DimRegistry::new(dims_name);
            let registry = UnitRegistry::new("test-ureg", dims);
            assert_eq!(registry.dims().name(), dims_name);
        }
    }

    mod new_with_meta {
        use super::*;

        #[test]
        fn stores_explicit_version() {
            let version = "42";
            let dims = DimRegistry::new("test-reg");
            let registry = UnitRegistry::new_with_meta("test-ureg", version, dims);
            assert_eq!(registry.version(), version);
        }
    }

    mod add_unit {
        use crate::test_utils::errors_match;

        use super::*;

        #[test]
        fn registers_atom_and_returns_matching_unit() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry
                .add_unit("meter", "m", length.clone(), 1.0)
                .unwrap();
            assert_eq!(meter.dimension(), &length);
        }

        #[test]
        fn stores_atom_with_correct_metadata() {
            // This test should be improved once I add the logic for the different conversion
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_unit("meter", "m", length.clone(), 2.0)
                .unwrap();
            let meter = registry.atoms.get("meter").unwrap();
            assert_eq!(meter.registry_id, registry.id());
            assert_eq!(meter.name, "meter".into());
            assert_eq!(meter.symbol, "m".into());
            assert_eq!(meter.dimension, length);
            assert_eq!(meter.conversion, ConversionKind::Linear { scale: 2.0 });
        }

        #[test]
        fn rejects_duplicate_name() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_unit("meter", "m", length.clone(), 1.0)
                .unwrap();
            let err = registry
                .add_unit("meter", "M", length.clone(), 1.0)
                .unwrap_err();
            let expected_err = UnitError::DuplicateName {
                name: "meter".into(),
                registry: registry.name().into(),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn allows_duplicate_symbols() {
            // Should never happen, but this is the current behavior
            // until I guard against duplicate symbols
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_unit("meter", "m", length.clone(), 1.0)
                .unwrap();
            let duplicate_symbol = registry.add_unit("Meter", "m", length.clone(), 1.0);
            assert!(duplicate_symbol.is_ok());
        }
    }
}
