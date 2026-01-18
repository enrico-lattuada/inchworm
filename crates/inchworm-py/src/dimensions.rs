use pyo3::prelude::*;
use pyo3::types::PyString;

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

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name: Bound<'_, PyString> = slf.get_type().qualname()?;
        Ok(format!("{}()", class_name))
    }
    fn __str__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name: Bound<'_, PyString> = slf.get_type().qualname()?;
        Ok(format!("{}", class_name))
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
