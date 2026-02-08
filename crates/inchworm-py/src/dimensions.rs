use inchworm::dimensions::{BaseDimensionDef, DimensionRegistry};
use pyo3::prelude::*;
use pyo3::types::PyString;

/// A definition of a base physical dimension.
#[pyclass(name = "BaseDimensionDef")]
pub struct PyBaseDimensionDef {
    _inner: BaseDimensionDef,
}

impl From<BaseDimensionDef> for PyBaseDimensionDef {
    /// Creates a `PyBaseDimensionDef` from a `BaseDimensionDef`.
    fn from(def: BaseDimensionDef) -> Self {
        PyBaseDimensionDef { _inner: def }
    }
}

#[pymethods]
impl PyBaseDimensionDef {
    /// Creates a new `BaseDimensionDef` with the given name and symbol.
    #[new]
    #[pyo3(text_signature = "(name, symbol)")]
    fn new(name: &str, symbol: &str) -> Self {
        BaseDimensionDef::new(name, symbol).into()
    }

    /// The name of the base dimension.
    #[getter]
    fn name(&self) -> &str {
        self._inner.name()
    }

    /// The symbol of the base dimension.
    #[getter]
    fn symbol(&self) -> &str {
        self._inner.symbol()
    }

    /// Returns a string representation of the base dimension definition.
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name: Bound<'_, PyString> = slf.get_type().qualname()?;
        let this = slf.borrow();
        let name = this.name();
        let symbol = this.symbol();
        Ok(format!(
            "{}(name='{}', symbol='{}')",
            class_name, name, symbol
        ))
    }

    /// Returns a string representation of the base dimension definition.
    fn __str__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let this = slf.borrow();
        let name = this.name();
        let symbol = this.symbol();
        Ok(format!("{} ([{}])", name, symbol))
    }
}

/// A registry for managing dimensions.
#[pyclass(name = "DimensionRegistry")]
pub struct PyDimensionRegistry {
    _inner: DimensionRegistry,
}

impl From<DimensionRegistry> for PyDimensionRegistry {
    /// Creates a `PyDimensionRegistry` from a `DimensionRegistry`.
    fn from(registry: DimensionRegistry) -> Self {
        PyDimensionRegistry { _inner: registry }
    }
}

#[pymethods]
impl PyDimensionRegistry {
    /// Creates a new, empty `DimensionRegistry`.
    #[new]
    fn new() -> Self {
        DimensionRegistry::new().into()
    }

    /// Returns a string representation of the dimension registry.
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name: Bound<'_, PyString> = slf.get_type().qualname()?;
        Ok(format!("{}()", class_name))
    }

    /// Returns a string representation of the dimension registry.
    fn __str__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name: Bound<'_, PyString> = slf.get_type().qualname()?;
        Ok(format!("{}", class_name))
    }
}

/// Unit tests for the Python bindings of the dimensions module.
#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the creation of a `PyBaseDimensionDef` instance.
    ///
    /// Verifies that the name and symbol are correctly stored and accessible.
    #[test]
    fn test_base_dimension_def_creation() {
        let dimension = PyBaseDimensionDef::new("length", "L");
        assert_eq!(dimension.name(), "length");
        assert_eq!(dimension.symbol(), "L");
    }

    /// Tests the creation of an empty `PyDimensionRegistry`.
    #[test]
    fn test_dimension_registry_creation() {
        let _registry = PyDimensionRegistry::new();
    }
}
