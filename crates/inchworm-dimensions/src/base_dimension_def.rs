use crate::errors::DimensionError;

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
/// let dimension = BaseDimensionDef::new("length", "L").unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct BaseDimensionDef {
    /// The name of the base dimension (e.g., "length", "mass").
    name: String,
    /// A symbol for the base dimension (e.g., "L" for length).
    symbol: String,
}

impl BaseDimensionDef {
    /// Creates a new `BaseDimensionDef` with the given name and symbol.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::InvalidDefinition`] if the name or symbol is empty.
    pub fn new(name: &str, symbol: &str) -> Result<Self, DimensionError> {
        if name.is_empty() {
            return Err(DimensionError::InvalidDefinition(
                "Base dimension name cannot be empty.".to_string(),
            ));
        }
        if symbol.is_empty() {
            return Err(DimensionError::InvalidDefinition(
                format!("Base dimension ({}) symbol cannot be empty.", name).to_string(),
            ));
        }
        Ok(Self {
            name: name.to_string(),
            symbol: symbol.to_string(),
        })
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
        let dimension = BaseDimensionDef::new("Length", "L").unwrap();
        assert_eq!(dimension.name, "Length");
        assert_eq!(dimension.symbol, "L");
    }

    // Test creation of BaseDimensionDef with a non-ASCII symbol
    #[test]
    fn test_base_dimension_with_non_ascii_symbol() {
        let dimension = BaseDimensionDef::new("Time", "τ").unwrap();
        assert_eq!(dimension.name, "Time");
        assert_eq!(dimension.symbol, "τ");
    }

    // Test BaseDimensionDef creation with empty name
    #[test]
    fn test_base_dimension_def_empty_name() {
        let result = BaseDimensionDef::new("", "L");
        assert!(matches!(result, Err(DimensionError::InvalidDefinition(_))));
    }

    // Test BaseDimensionDef creation with empty symbol
    #[test]
    fn test_base_dimension_def_empty_symbol() {
        let result = BaseDimensionDef::new("Length", "");
        assert!(matches!(result, Err(DimensionError::InvalidDefinition(_))));
    }

    // Test name() method
    #[test]
    fn test_base_dimension_get_name() {
        let dimension = BaseDimensionDef::new("Mass", "M").unwrap();
        assert_eq!(dimension.name(), "Mass");
    }

    // Test symbol() method
    #[test]
    fn test_base_dimension_get_symbol() {
        let dimension = BaseDimensionDef::new("Current", "I").unwrap();
        assert_eq!(dimension.symbol(), "I");
    }
}
