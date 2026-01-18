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
        "DimensionRegistry()".to_string()
    }
    fn __str__(&self) -> String {
        "DimensionRegistry".to_string()
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
