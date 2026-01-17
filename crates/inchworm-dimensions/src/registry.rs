/// A registry for managing dimensions in a units system.
///
/// `DimensionRegistry` provides a central location to define and manage
/// physical dimensions (e.g., length, mass, time) that form the foundation
/// of the units system.
///
/// # Examples
///
/// ```
/// use inchworm_dimensions::registry::DimensionRegistry;
///
/// let registry = DimensionRegistry::new();
/// ```
#[derive(Debug)]
pub struct DimensionRegistry {}

impl DimensionRegistry {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimension_registry_creation() {
        let _registry = DimensionRegistry::new();
    }
}
