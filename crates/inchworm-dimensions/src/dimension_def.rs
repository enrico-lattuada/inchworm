use crate::base_dimension_def::BaseDimensionDef;
use crate::derived_dimension_def::DerivedDimensionDef;

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
    /// The name of the dimension.
    pub fn name(&self) -> &str {
        match self {
            DimensionDef::Base(def) => def.name(),
            DimensionDef::Derived(def) => def.name(),
        }
    }

    /// The symbol of the dimension.
    pub fn symbol(&self) -> &str {
        match self {
            DimensionDef::Base(def) => def.symbol(),
            DimensionDef::Derived(def) => def.symbol(),
        }
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
