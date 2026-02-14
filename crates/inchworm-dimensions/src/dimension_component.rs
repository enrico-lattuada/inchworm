use num_rational::Ratio;
use std::sync::{Arc, Weak};

use crate::{dimension_def::DimensionDef, errors::DimensionError};

/// A component of a derived dimension.
///
/// `DimensionComponent` represents a single component of a derived dimension,
/// consisting of a reference to another [`DimensionDef`] (which can be either
/// a base or derived dimension) and an exponent that indicates how that
/// dimension contributes to the overall derived dimension. For example, in the
/// derived dimension of velocity (length/time), you would have a
/// `DimensionComponent` for length with an exponent of 1 and a
/// `DimensionComponent` for time with an exponent of -1.
#[derive(Debug, Clone)]
pub struct DimensionComponent {
    /// A weak reference to the dimension that is a component of the derived
    /// dimension.
    dimension: Weak<DimensionDef>,
    /// The exponent that indicates how the dimension contributes to the
    /// overall derived dimension.
    exponent: Ratio<i32>,
}

impl DimensionComponent {
    /// Creates a new `DimensionComponent` with the given dimension and exponent.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::InvalidComponent`] if:
    /// - The dimension reference has been dropped (weak reference is no longer valid).
    /// - The exponent is zero.
    pub fn new(
        dimension: Weak<DimensionDef>,
        exponent: Ratio<i32>,
    ) -> Result<Self, DimensionError> {
        if dimension.upgrade().is_none() {
            return Err(DimensionError::InvalidComponent(
                "Cannot create DimensionComponent when the dimension reference has been dropped."
                    .to_string(),
            ));
        }
        if exponent == Ratio::from(0) {
            return Err(DimensionError::InvalidComponent(
                "Exponent cannot be zero. Consider referencing a dimensionless dimension with an exponent of 1.".to_string(),
            ));
        }
        Ok(Self {
            dimension,
            exponent,
        })
    }

    /// Whether the dimension reference is still valid (i.e., the dimension has
    /// not been dropped).
    pub fn is_valid(&self) -> bool {
        self.dimension.upgrade().is_some()
    }

    /// Returns an [`Arc`] to the [`DimensionDef`] of the component if it is
    /// still valid, or `None` if it has been dropped.
    pub fn dimension(&self) -> Option<Arc<DimensionDef>> {
        self.dimension.upgrade()
    }

    /// Returns the exponent of the component.
    pub fn exponent(&self) -> Ratio<i32> {
        // This returns a copy of the exponent
        self.exponent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base_dimension_def::BaseDimensionDef;

    // Test creation of DimensionComponent
    #[test]
    fn test_dimension_component_creation() {
        let dimension = Arc::new(BaseDimensionDef::new("Length", "L").unwrap().into());
        let _component = DimensionComponent::new(Arc::downgrade(&dimension), Ratio::from(1));
    }

    // Test creation of DimensionComponent with invalid dimension reference
    #[test]
    fn test_dimension_component_creation_invalid_dimension() {
        let dimension = Arc::new(BaseDimensionDef::new("Length", "L").unwrap().into());
        let weak_dimension = Arc::downgrade(&dimension);
        drop(dimension); // Drop the original dimension to make the weak reference invalid
        let result = DimensionComponent::new(weak_dimension, Ratio::from(1));
        assert!(matches!(result, Err(DimensionError::InvalidComponent(_))));
    }

    // Test creation of DimensionComponent with zero exponent
    #[test]
    fn test_dimension_component_creation_zero_exponent() {
        let dimension = Arc::new(BaseDimensionDef::new("Length", "L").unwrap().into());
        let result = DimensionComponent::new(Arc::downgrade(&dimension), Ratio::from(0));
        assert!(matches!(result, Err(DimensionError::InvalidComponent(_))));
    }

    // Test is_valid() method
    #[test]
    fn test_dimension_component_is_valid() {
        let dimension = Arc::new(BaseDimensionDef::new("Length", "L").unwrap().into());
        let component =
            DimensionComponent::new(Arc::downgrade(&dimension), Ratio::from(1)).unwrap();
        assert!(component.is_valid());

        // Drop the original dimension
        drop(dimension);
        assert!(!component.is_valid());
    }

    // Test dimension() method
    #[test]
    fn test_dimension_component_dimension() {
        let dimension = Arc::new(BaseDimensionDef::new("Length", "L").unwrap().into());
        let component =
            DimensionComponent::new(Arc::downgrade(&dimension), Ratio::from(1)).unwrap();
        assert!(component.dimension().is_some());
        assert_eq!(component.dimension().unwrap().name(), "Length");
    }

    // Test exponent() method
    #[test]
    fn test_dimension_component_exponent() {
        let dimension = Arc::new(BaseDimensionDef::new("Length", "L").unwrap().into());
        let component =
            DimensionComponent::new(Arc::downgrade(&dimension), Ratio::new(3, 2)).unwrap();
        assert_eq!(component.exponent(), Ratio::new(3, 2));
    }
}
