use crate::base_dimension_def::BaseDimensionDef;
use crate::derived_dimension_def::DerivedDimensionDef;
use crate::dimension_component::DimensionComponent;

/// A physical dimension definition, either base or derived.
///
/// `DimensionDef` is the central type used to represent dimensions in the system.
/// Base dimensions (e.g., length, mass, time) are fundamental, while derived
/// dimensions (e.g., velocity, force) are formed by combining base or other
/// derived dimensions with rational exponents.
#[derive(Debug, Clone)]
pub enum DimensionDef {
    /// A base dimension definition, representing fundamental physical
    /// dimensions.
    Base(BaseDimensionDef),
    /// A derived dimension definition, representing dimensions formed by
    /// combining base or other derived dimensions.
    Derived(DerivedDimensionDef),
}

impl DimensionDef {
    /// Returns the name of the dimension.
    pub fn name(&self) -> &str {
        match self {
            DimensionDef::Base(def) => def.name(),
            DimensionDef::Derived(def) => def.name(),
        }
    }

    /// Returns the symbol of the dimension.
    pub fn symbol(&self) -> &str {
        match self {
            DimensionDef::Base(def) => def.symbol(),
            DimensionDef::Derived(def) => def.symbol(),
        }
    }

    /// Returns the components of the dimension. For base dimensions, this will
    /// be an empty vector. For derived dimensions, this will be a vector of
    /// the components that make up the derived dimension.
    pub fn components(&self) -> &[DimensionComponent] {
        match self {
            DimensionDef::Base(_) => &[],
            DimensionDef::Derived(def) => def.components(),
        }
    }

    /// Whether this is a base dimension.
    pub fn is_base(&self) -> bool {
        matches!(self, DimensionDef::Base(_))
    }

    /// Whether this is a derived dimension.
    pub fn is_derived(&self) -> bool {
        matches!(self, DimensionDef::Derived(_))
    }
}

/// Creates a [`DimensionDef::Base`] from a [`BaseDimensionDef`].
impl From<BaseDimensionDef> for DimensionDef {
    fn from(def: BaseDimensionDef) -> Self {
        DimensionDef::Base(def)
    }
}

/// Creates a [`DimensionDef::Derived`] from a [`DerivedDimensionDef`].
impl From<DerivedDimensionDef> for DimensionDef {
    fn from(def: DerivedDimensionDef) -> Self {
        DimensionDef::Derived(def)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base_dimension_def::BaseDimensionDef;
    use num_rational::Ratio;
    use std::sync::Arc;

    // Test creation of DimensionDef from BaseDimensionDef
    #[test]
    fn test_dimension_def_from_base() {
        let base_def = BaseDimensionDef::new("Length", "L").unwrap();
        let dimension_def = DimensionDef::from(base_def);
        assert!(dimension_def.is_base());
        assert_eq!(dimension_def.name(), "Length");
        assert_eq!(dimension_def.symbol(), "L");
        assert!(dimension_def.components().is_empty());
    }

    // Test creation of DimensionDef from DerivedDimensionDef
    #[test]
    fn test_dimension_def_from_derived() {
        let length = Arc::new(BaseDimensionDef::new("Length", "L").unwrap().into());
        let time = Arc::new(BaseDimensionDef::new("Time", "T").unwrap().into());
        let derived_def = DerivedDimensionDef::new(
            "Velocity",
            "v",
            vec![
                DimensionComponent::new(Arc::downgrade(&length), Ratio::from(1)).unwrap(),
                DimensionComponent::new(Arc::downgrade(&time), Ratio::from(-1)).unwrap(),
            ],
        )
        .unwrap();
        let dimension_def = DimensionDef::from(derived_def);
        assert!(dimension_def.is_derived());
        assert_eq!(dimension_def.name(), "Velocity");
        assert_eq!(dimension_def.symbol(), "v");
        let components = dimension_def.components();
        assert_eq!(components.len(), 2);
    }
}
