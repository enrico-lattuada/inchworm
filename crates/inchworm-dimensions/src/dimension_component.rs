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

    /// Returns a reference to the [`DimensionDef`] of the component.
    pub fn dimension(&self) -> &Weak<DimensionDef> {
        &self.dimension
    }

    /// Returns an [`Arc`] to the [`DimensionDef`] of the component if it is
    /// still valid, or `None` if it has been dropped.
    pub fn dimension_def(&self) -> Option<Arc<DimensionDef>> {
        self.dimension.upgrade()
    }

    /// Returns the exponent of the component.
    pub fn exponent(&self) -> Ratio<i32> {
        // This returns a copy of the exponent
        self.exponent
    }
}
