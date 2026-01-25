/// A definition of a base physical dimension.
///
/// `BaseDimensionDef` represents fundamental physical dimensions such as
/// length, mass, and time, that form the basis for derived dimensions in a
/// units system.
/// Each base dimension has a name and an optional symbol for compact
/// representation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BaseDimensionDef {
    // The name of the base dimension (e.g., "length", "mass").
    name: String,
    // An optional symbol for the base dimension (e.g., "L" for length).
    symbol: Option<String>,
}

impl BaseDimensionDef {
    /// Creates a new `BaseDimensionDef` with the given name and optional symbol.
    /// The name is case-insensitive and will be stored in lowercase.
    pub fn new(name: &str, symbol: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            symbol: symbol.map(|s| s.to_string()),
        }
    }

    /// Returns the name of the base dimension.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Returns the symbol of the base dimension, if it exists.
    pub fn get_symbol(&self) -> Option<&str> {
        self.symbol.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test creation of BaseDimensionDef
    #[test]
    fn test_base_dimension_def_creation() {
        let dimension = BaseDimensionDef::new("length", Some("L"));
        assert_eq!(dimension.name, "length");
        assert_eq!(dimension.symbol.as_deref(), Some("L"));
    }

    // Test creation of BaseDimensionDef without symbol
    #[test]
    fn test_base_dimension_with_non_ascii_symbol() {
        let dimension = BaseDimensionDef::new("time", Some("τ"));
        assert_eq!(dimension.name, "time");
        assert_eq!(dimension.symbol.as_deref(), Some("τ"));
    }

    // Test BaseDimensionDef get_name method
    #[test]
    fn test_get_name() {
        let dimension = BaseDimensionDef::new("mass", None);
        assert_eq!(dimension.get_name(), "mass");
    }

    // Test BaseDimensionDef get_symbol method
    #[test]
    fn test_get_symbol() {
        let dimension = BaseDimensionDef::new("current", Some("I"));
        assert_eq!(dimension.get_symbol(), Some("I"));
    }
}
