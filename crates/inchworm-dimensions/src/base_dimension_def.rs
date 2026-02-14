/// A definition of a base physical dimension.
///
/// `BaseDimensionDef` represents fundamental physical dimensions such as
/// length, mass, and time, that form the basis for derived dimensions in a
/// units system.
///
/// # Examples
///
/// ```
/// use inchworm_dimensions::BaseDimensionDef;
///
/// let dimension = BaseDimensionDef::new("length", "L");
/// ```
#[derive(Debug, Clone)]
pub struct BaseDimensionDef {
    // The name of the base dimension (e.g., "length", "mass").
    name: String,
    // A symbol for the base dimension (e.g., "L" for length).
    symbol: String,
}

impl BaseDimensionDef {
    /// Creates a new `BaseDimensionDef` with the given name and symbol.
    pub fn new(name: &str, symbol: &str) -> Self {
        // TODO: Raise error if name or symbol are empty
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
        let dimension = BaseDimensionDef::new("Length", "L");
        assert_eq!(dimension.name, "Length");
        assert_eq!(dimension.symbol, "L");
    }

    // Test creation of BaseDimensionDef with a non-ASCII symbol
    #[test]
    fn test_base_dimension_with_non_ascii_symbol() {
        let dimension = BaseDimensionDef::new("Time", "τ");
        assert_eq!(dimension.name, "Time");
        assert_eq!(dimension.symbol, "τ");
    }

    // Test BaseDimensionDef name method
    #[test]
    fn test_base_dimension_get_name() {
        let dimension = BaseDimensionDef::new("Mass", "M");
        assert_eq!(dimension.name(), "Mass");
    }

    // Test BaseDimensionDef symbol method
    #[test]
    fn test_base_dimension_get_symbol() {
        let dimension = BaseDimensionDef::new("Current", "I");
        assert_eq!(dimension.symbol(), "I");
    }
}
