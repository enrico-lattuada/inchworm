use crate::dimension_def::BaseDimensionDef;
use std::{collections::HashMap, error, fmt};

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
    pub fn base_dimensions(&self) -> &HashMap<String, BaseDimensionDef> {
        &self.base_dimensions
    }

    /// Inserts a new base dimension in the registry.
    /// Returns an error if a base dimension with the same name already exists.
    pub fn try_insert_new_base_dimension(
        &mut self,
        dimension: &str,
        definition: BaseDimensionDef,
    ) -> Result<(), DimensionRegistryError> {
        if self.base_dimensions.contains_key(dimension) {
            Err(DimensionRegistryError::DuplicateBaseDimension {
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
        self.base_dimensions
            .insert(dimension.to_string(), definition)
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum DimensionRegistryError {
    /// Error indicating that a base dimension with the same name already exists.
    DuplicateBaseDimension { dimension: String },
}

impl error::Error for DimensionRegistryError {}

impl fmt::Display for DimensionRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DimensionRegistryError::DuplicateBaseDimension { dimension } => {
                write!(
                    f,
                    "base dimension '{}' already exists in the registry",
                    dimension
                )
            }
        }
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

    /// Test try insert a new base dimension
    #[test]
    fn test_try_insert_new_base_dimension() {
        let mut registry = DimensionRegistry::new();
        let dimension = BaseDimensionDef::new("length", Some("L"));
        assert!(
            registry
                .try_insert_new_base_dimension("length", dimension)
                .is_ok(),
            "Failed to register base dimension"
        );
    }

    /// Test try insert a new base dimension with the same name (case-insensitive)
    #[test]
    fn test_try_insert_existing_base_dimension() {
        let mut registry = DimensionRegistry::new();
        let dimension1 = BaseDimensionDef::new("length", Some("L"));
        let dimension2 = BaseDimensionDef::new("Length", Some("Len"));
        registry
            .try_insert_new_base_dimension("length", dimension1.clone())
            .unwrap();
        let res = registry.try_insert_new_base_dimension("length", dimension2);
        assert!(
            matches!(
                res,
                Err(DimensionRegistryError::DuplicateBaseDimension { .. })
            ),
            "Expected error when registering base dimension with duplicate name"
        );
        assert!(
            registry.base_dimensions().get("length") == Some(&dimension1),
            "Original base dimension should remain unchanged"
        );
    }

    /// Test retrieving a registered base dimension
    #[test]
    fn test_replace_base_dimension() {
        let mut registry = DimensionRegistry::new();
        let dimension1 = BaseDimensionDef::new("length", Some("L"));
        let dimension2 = BaseDimensionDef::new("Length", Some("Len"));
        registry
            .try_insert_new_base_dimension("length", dimension1)
            .unwrap();
        registry.replace_base_dimension("length", dimension2.clone());
        assert_eq!(
            registry.base_dimensions().len(),
            1,
            "Expected only one base dimension after replacement"
        );
        assert_eq!(
            registry.base_dimensions().get("length"),
            Some(&dimension2),
            "Base dimension was not replaced correctly"
        );
    }
}
