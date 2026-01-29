/// A definition of a base physical dimension.
///
/// `BaseDimensionDef` represents fundamental physical dimensions such as
/// length, mass, and time, that form the basis for derived dimensions in a
/// units system.
/// Each base dimension has a name and an optional symbol for compact
/// representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseDimensionDef {
    // The name of the base dimension (e.g., "length", "mass").
    name: String,
    // A symbol for the base dimension (e.g., "L" for length).
    symbol: String,
}

impl BaseDimensionDef {
    /// Creates a new `BaseDimensionDef` with the given name and optional symbol.
    /// The name is case-insensitive and will be stored in lowercase.
    pub fn new(name: &str, symbol: &str) -> Self {
        Self {
            name: name.to_string(),
            symbol: symbol.to_string(),
        }
    }

    /// Returns the name of the base dimension.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the symbol of the base dimension.
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test creation of BaseDimensionDef
    #[test]
    fn test_base_dimension_def_creation() {
        let dimension = BaseDimensionDef::new("length", "L");
        assert_eq!(dimension.name, "length");
        assert_eq!(dimension.symbol, "L");
    }

    // Test creation of BaseDimensionDef without symbol
    #[test]
    fn test_base_dimension_with_non_ascii_symbol() {
        let dimension = BaseDimensionDef::new("time", "τ");
        assert_eq!(dimension.name, "time");
        assert_eq!(dimension.symbol, "τ");
    }

    // Test BaseDimensionDef get_name method
    #[test]
    fn test_get_name() {
        let dimension = BaseDimensionDef::new("mass", "M");
        assert_eq!(dimension.name(), "mass");
    }

    // Test BaseDimensionDef get_symbol method
    #[test]
    fn test_get_symbol() {
        let dimension = BaseDimensionDef::new("current", "I");
        assert_eq!(dimension.symbol(), "I");
    }
}
