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

/// A definition of a derived physical dimension.
///
/// `DerivedDimensionDef` represents derived physical dimensions that are
/// formed by combining base or other derived dimensions in a units system.
///
/// # Examples
///
/// ```
/// use inchworm_dimensions::DerivedDimensionDef;
///
/// let dimension = DerivedDimensionDef::new("length", "L");
/// ```
#[derive(Debug, Clone)]
pub struct DerivedDimensionDef {
    // The name of the derived dimension (e.g., "velocity", "acceleration").
    name: String,
    // A symbol for the derived dimension (e.g., "V" for velocity).
    symbol: String,
}

impl DerivedDimensionDef {
    /// Creates a new `DerivedDimensionDef` with the given name and symbol.
    pub fn new(name: &str, symbol: &str) -> Self {
        Self {
            name: name.to_string(),
            symbol: symbol.to_string(),
        }
    }

    /// Returns the name of the derived dimension.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the symbol of the derived dimension.
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

    // Test BaseDimensionDef get_name method
    #[test]
    fn test_base_dimension_get_name() {
        let dimension = BaseDimensionDef::new("Mass", "M");
        assert_eq!(dimension.name(), "Mass");
    }

    // Test BaseDimensionDef get_symbol method
    #[test]
    fn test_base_dimension_get_symbol() {
        let dimension = BaseDimensionDef::new("Current", "I");
        assert_eq!(dimension.symbol(), "I");
    }

    // Test creation of DerivedDimensionDef
    #[test]
    fn test_derived_dimension_def_creation() {
        let dimension = DerivedDimensionDef::new("Velocity", "v");
        assert_eq!(dimension.name, "Velocity");
        assert_eq!(dimension.symbol, "v");
    }

    // Test creation of DerivedDimensionDef with a non-ASCII symbol
    #[test]
    fn test_derived_dimension_with_non_ascii_symbol() {
        // Temperature uses capital Theta
        let dimension = DerivedDimensionDef::new("Temperature", "Θ");
        assert_eq!(dimension.name, "Temperature");
        assert_eq!(dimension.symbol, "Θ");
    }

    // Test DerivedDimensionDef get_name method
    #[test]
    fn test_derived_dimension_get_name() {
        let dimension = DerivedDimensionDef::new("Velocity", "v");
        assert_eq!(dimension.name(), "Velocity");
    }

    // Test DerivedDimensionDef get_symbol method
    #[test]
    fn test_derived_dimension_get_symbol() {
        let dimension = DerivedDimensionDef::new("Velocity", "v");
        assert_eq!(dimension.symbol(), "v");
    }
}
