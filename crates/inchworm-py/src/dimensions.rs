use inchworm::dimensions::DimensionRegistry;
use pyo3::prelude::*;
use pyo3::types::PyString;

/// DimensionRegistry for managing dimensions.
#[pyclass(name = "DimensionRegistry")]
pub struct PyDimensionRegistry {
    _inner: DimensionRegistry,
}

#[pymethods]
impl PyDimensionRegistry {
    #[new]
    fn new() -> Self {
        PyDimensionRegistry {
            _inner: DimensionRegistry::new(),
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
        let _registry = PyDimensionRegistry::new();
    }
}
