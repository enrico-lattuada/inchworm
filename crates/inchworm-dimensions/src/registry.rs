use crate::{RegistryError, dimension_def::BaseDimensionDef};
use std::collections::HashMap;

/// A registry for managing dimensions in a units system.
///
/// `DimensionRegistry` provides a central location to define and manage
/// physical dimensions (e.g., length, mass, time) that form the foundation
/// of the units system.
///
/// # Examples
///
/// ```
/// use inchworm_dimensions::DimensionRegistry;
///
/// let registry = DimensionRegistry::new();
/// ```
#[derive(Debug)]
pub struct DimensionRegistry {
    base_dimensions: HashMap<String, BaseDimensionDef>,
}

impl DimensionRegistry {
    /// Creates a new, empty `DimensionRegistry`.
    pub fn new() -> Self {
        Self {
            base_dimensions: HashMap::new(),
        }
    }

    /// Retrieves all registered base dimensions in the registry.
    pub fn get_base_dimensions(&self) -> &HashMap<String, BaseDimensionDef> {
        &self.base_dimensions
    }

    /// Registers a new base dimension in the registry.
    /// Returns an error if a base dimension with the same name already exists.
    pub fn register_base_dimension(
        &mut self,
        dimension: &str,
        definition: BaseDimensionDef,
    ) -> Result<(), RegistryError> {
        if self.base_dimensions.contains_key(dimension) {
            Err(RegistryError::BaseDimensionAlreadyDefined {
                dimension: dimension.to_string(),
            })
        } else {
            self.base_dimensions
                .insert(dimension.to_string(), definition);
            Ok(())
        }
    }

    /// Replaces an existing base dimension with the same name in the registry.
    /// Returns the previous definition if it existed.
    pub fn replace_base_dimension(
        &mut self,
        dimension: &str,
        definition: BaseDimensionDef,
    ) -> Option<BaseDimensionDef> {
        let previous_def = self.base_dimensions.get(dimension).cloned();
        self.base_dimensions
            .insert(dimension.to_string(), definition);
        previous_def
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test the creation of a DimensionRegistry
    #[test]
    fn test_dimension_registry_creation() {
        let _registry = DimensionRegistry::new();
    }

    /// Test registering a base dimension
    #[test]
    fn test_register_base_dimension() {
        let mut registry = DimensionRegistry::new();
        let dimension = BaseDimensionDef::new("length", Some("L"));
        assert!(
            registry
                .register_base_dimension("length", dimension)
                .is_ok(),
            "Failed to register base dimension"
        );
    }

    /// Test registering a base dimension with the same name (case-insensitive)
    #[test]
    fn test_register_base_dimension_same_key() {
        let mut registry = DimensionRegistry::new();
        let dimension1 = BaseDimensionDef::new("length", Some("L"));
        let dimension2 = BaseDimensionDef::new("Length", Some("Len"));
        registry
            .register_base_dimension("length", dimension1)
            .unwrap();
        assert!(
            matches!(
                registry.register_base_dimension("length", dimension2),
                Err(RegistryError::BaseDimensionAlreadyDefined { .. })
            ),
            "Expected error when registering base dimension with duplicate name"
        );
    }

    /// Test retrieving a registered base dimension
    #[test]
    fn test_replace_base_dimension() {
        let mut registry = DimensionRegistry::new();
        let dimension1 = BaseDimensionDef::new("length", Some("L"));
        let dimension2 = BaseDimensionDef::new("Length", Some("Len"));
        registry
            .register_base_dimension("length", dimension1)
            .unwrap();
        registry.replace_base_dimension("length", dimension2.clone());
        assert_eq!(
            registry.get_base_dimensions().len(),
            1,
            "Expected only one base dimension after replacement"
        );
        assert_eq!(
            registry.get_base_dimensions().get("length"),
            Some(&dimension2),
            "Base dimension was not replaced correctly"
        );
    }
}
