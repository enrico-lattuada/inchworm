//! The prebuilt standard-dimensions registry.
//!
//! Built by parsing the embedded `data/standard.toml`. Infallibility is guaranteed by a unit test.

use crate::DimRegistry;

pub(crate) const STANDARD_TOML: &str = include_str!("../data/standard.toml");

impl DimRegistry {
    /// A fresh, independently mutable copy of the standard-dimensions
    /// registry (ISQ base dimensions, common named derived dimensions,
    /// plane/solid angle as irreducible dimensionless kinds, ...).
    pub fn standard() -> Self {
        Self::from_toml_str(STANDARD_TOML).expect("standard registry must be loaded correctly")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_standard() {
        let registry = DimRegistry::standard();
        assert!(registry.get("velocity").is_some());
        assert!(registry.get("plane_angle").is_some());
    }
}
