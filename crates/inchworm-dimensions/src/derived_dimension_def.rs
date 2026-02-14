use crate::{dimension_component::DimensionComponent, errors::DimensionError};

/// A definition of a derived physical dimension.
///
/// `DerivedDimensionDef` represents derived physical dimensions that are
/// formed by combining base or other derived dimensions in a units system.
///
/// # Examples
///
/// ```
/// use inchworm_dimensions::{BaseDimensionDef, DerivedDimensionDef, DimensionComponent};
/// use num_rational::Ratio;
/// use std::sync::Arc;
///
/// let length = Arc::new(BaseDimensionDef::new("Length", "L").unwrap().into());
/// let time = Arc::new(BaseDimensionDef::new("Time", "T").unwrap().into());
/// let _velocity = DerivedDimensionDef::new(
///     "Velocity",
///     "v",
///     vec![
///         DimensionComponent::new(Arc::downgrade(&length), Ratio::from(1)).unwrap(),
///         DimensionComponent::new(Arc::downgrade(&time), Ratio::from(-1)).unwrap(),
///     ],
/// ).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct DerivedDimensionDef {
    /// The name of the derived dimension (e.g., "velocity", "acceleration").
    name: String,
    /// A symbol for the derived dimension (e.g., "V" for velocity).
    symbol: String,
    /// Components whose product forms the derived dimension
    components: Vec<DimensionComponent>,
}

impl DerivedDimensionDef {
    /// Creates a new `DerivedDimensionDef` with the given name and symbol.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::InvalidDefinition`] if the name or symbol is empty, or if there are no components.
    pub fn new(
        name: &str,
        symbol: &str,
        components: Vec<DimensionComponent>,
    ) -> Result<Self, DimensionError> {
        if name.is_empty() {
            return Err(DimensionError::InvalidDefinition(
                "Derived dimension name cannot be empty.".to_string(),
            ));
        }
        if symbol.is_empty() {
            return Err(DimensionError::InvalidDefinition(
                format!("Derived dimension ({}) symbol cannot be empty.", name).to_string(),
            ));
        }
        if components.is_empty() {
            return Err(DimensionError::InvalidDefinition(
                format!(
                    "Derived dimension ({}) must have at least one component.",
                    name
                )
                .to_string(),
            ));
        }
        if components.iter().any(|c| !c.is_valid()) {
            return Err(DimensionError::InvalidDefinition(
                format!(
                    "Derived dimension ({}) has invalid components. All component dimension references must be valid.",
                    name
                )
                .to_string(),
            ));
        }
        Ok(Self {
            name: name.to_string(),
            symbol: symbol.to_string(),
            components,
        })
    }

    /// Returns the name of the derived dimension.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the symbol of the derived dimension.
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Returns the components of the derived dimension.
    pub fn components(&self) -> &[DimensionComponent] {
        &self.components
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DimensionDef, base_dimension_def::BaseDimensionDef};
    use num_rational::Ratio;
    use std::sync::Arc;

    // Helper function to create a base dimension wrapped in an Arc and converted to DimensionDef
    fn make_base_dimension(name: &str, symbol: &str) -> Arc<DimensionDef> {
        Arc::new(BaseDimensionDef::new(name, symbol).unwrap().into())
    }

    // Test creation of DerivedDimensionDef
    #[test]
    fn test_derived_dimension_def_creation() {
        let length = make_base_dimension("Length", "L");
        let time = make_base_dimension("Time", "T");
        let _velocity = DerivedDimensionDef::new(
            "Velocity",
            "v",
            vec![
                DimensionComponent::new(Arc::downgrade(&length), Ratio::from(1)).unwrap(),
                DimensionComponent::new(Arc::downgrade(&time), Ratio::from(-1)).unwrap(),
            ],
        );
    }

    // Test creation of DerivedDimensionDef with a non-ASCII symbol
    #[test]
    fn test_derived_dimension_with_non_ascii_symbol() {
        let length = make_base_dimension("Length", "L");
        let _strain = DerivedDimensionDef::new(
            "Strain",
            "ε",
            vec![
                DimensionComponent::new(Arc::downgrade(&length), Ratio::from(1)).unwrap(),
                DimensionComponent::new(Arc::downgrade(&length), Ratio::from(-1)).unwrap(),
            ],
        );
    }

    // Test creation of DerivedDimensionDef with empty name
    #[test]
    fn test_derived_dimension_with_empty_name() {
        let time = make_base_dimension("Time", "T");
        let result = DerivedDimensionDef::new(
            "",
            "f",
            vec![DimensionComponent::new(Arc::downgrade(&time), Ratio::from(-1)).unwrap()],
        );
        assert!(matches!(result, Err(DimensionError::InvalidDefinition(_))));
    }

    // Test creation of DerivedDimensionDef with empty symbol
    #[test]
    fn test_derived_dimension_with_empty_symbol() {
        let time = make_base_dimension("Time", "T");
        let result = DerivedDimensionDef::new(
            "Frequency",
            "",
            vec![DimensionComponent::new(Arc::downgrade(&time), Ratio::from(-1)).unwrap()],
        );
        assert!(matches!(result, Err(DimensionError::InvalidDefinition(_))));
    }

    // Test creation of DerivedDimensionDef with no components
    #[test]
    fn test_derived_dimension_with_no_components() {
        let result = DerivedDimensionDef::new("Dimensionless", "1", vec![]);
        assert!(matches!(result, Err(DimensionError::InvalidDefinition(_))));
    }

    // Test DerivedDimensionDef name method
    #[test]
    fn test_derived_dimension_get_name() {
        let length = make_base_dimension("Length", "L");
        let time = make_base_dimension("Time", "T");
        let velocity = DerivedDimensionDef::new(
            "Velocity",
            "v",
            vec![
                DimensionComponent::new(Arc::downgrade(&length), Ratio::from(1)).unwrap(),
                DimensionComponent::new(Arc::downgrade(&time), Ratio::from(-1)).unwrap(),
            ],
        )
        .unwrap();
        assert_eq!(velocity.name(), "Velocity");
    }

    // Test DerivedDimensionDef symbol method
    #[test]
    fn test_derived_dimension_get_symbol() {
        let length = make_base_dimension("Length", "L");
        let time = make_base_dimension("Time", "T");
        let velocity = DerivedDimensionDef::new(
            "Velocity",
            "v",
            vec![
                DimensionComponent::new(Arc::downgrade(&length), Ratio::from(1)).unwrap(),
                DimensionComponent::new(Arc::downgrade(&time), Ratio::from(-1)).unwrap(),
            ],
        )
        .unwrap();
        assert_eq!(velocity.symbol(), "v");
    }

    // Test DerivedDimensionDef components method
    #[test]
    fn test_derived_dimension_get_components() {
        let length = make_base_dimension("Length", "L");
        let time = make_base_dimension("Time", "T");
        let velocity = DerivedDimensionDef::new(
            "Velocity",
            "v",
            vec![
                DimensionComponent::new(Arc::downgrade(&length), Ratio::from(1)).unwrap(),
                DimensionComponent::new(Arc::downgrade(&time), Ratio::from(-1)).unwrap(),
            ],
        )
        .unwrap();
        let components = velocity.components();
        assert_eq!(components.len(), 2);
        assert_eq!(components[0].dimension_def().unwrap().name(), "Length");
        assert_eq!(components[0].exponent(), Ratio::from(1));
        assert_eq!(components[1].dimension_def().unwrap().name(), "Time");
        assert_eq!(components[1].exponent(), Ratio::from(-1));
    }
}
