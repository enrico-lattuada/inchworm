use pyo3::prelude::*;

/// DimensionRegistry for managing dimensions.
#[pyclass]
pub struct DimensionRegistry {
    _inner: inchworm::dimensions::DimensionRegistry,
}

#[pymethods]
impl DimensionRegistry {
    #[new]
    fn new() -> Self {
        DimensionRegistry {
            _inner: inchworm::dimensions::DimensionRegistry::new(),
        }
    }

    fn __repr__(&self) -> String {
        format!("DimensionRegistry()")
    }
    fn __str__(&self) -> String {
        format!("DimensionRegistry")
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
