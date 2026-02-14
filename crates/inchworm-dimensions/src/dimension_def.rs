use crate::base_dimension_def::BaseDimensionDef;
use crate::derived_dimension_def::DerivedDimensionDef;

/// A definition of a physical dimension.
#[derive(Debug, Clone)]
pub enum DimensionDef {
    /// A base dimension definition, representing fundamental physical dimensions.
    Base(BaseDimensionDef),
    /// A derived dimension definition, representing dimensions formed by combining base or other derived dimensions.
    Derived(DerivedDimensionDef),
}

impl DimensionDef {
    /// Returns the name of the dimension, whether it's a base or derived dimension.
    pub fn name(&self) -> &str {
        match self {
            DimensionDef::Base(def) => def.name(),
            DimensionDef::Derived(def) => def.name(),
        }
    }

    /// Returns the symbol of the dimension, whether it's a base or derived dimension.
    pub fn symbol(&self) -> &str {
        match self {
            DimensionDef::Base(def) => def.symbol(),
            DimensionDef::Derived(def) => def.symbol(),
        }
    }
}

impl From<BaseDimensionDef> for DimensionDef {
    fn from(def: BaseDimensionDef) -> Self {
        DimensionDef::Base(def)
    }
}

impl From<DerivedDimensionDef> for DimensionDef {
    fn from(def: DerivedDimensionDef) -> Self {
        DimensionDef::Derived(def)
    }
}
