use inchworm::dimensions::{BaseDimensionDef, DimensionRegistry};
use pyo3::prelude::*;
use pyo3::types::{PyIterator, PyList, PyString};

/// BaseDimensionDef represents a base dimension definition.
// Should this be frozen?
#[pyclass(name = "BaseDimensionDef")]
#[derive(Clone, PartialEq, Eq)]
pub struct PyBaseDimensionDef {
    _inner: BaseDimensionDef,
}

impl From<BaseDimensionDef> for PyBaseDimensionDef {
    fn from(def: BaseDimensionDef) -> Self {
        PyBaseDimensionDef { _inner: def }
    }
}

#[pymethods]
impl PyBaseDimensionDef {
    #[new]
    fn new(name: &str, symbol: &str) -> Self {
        BaseDimensionDef::new(name, symbol).into()
    }

    #[getter]
    fn name(&self) -> &str {
        self._inner.name()
    }

    #[getter]
    fn symbol(&self) -> &str {
        self._inner.symbol()
    }

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
}

/// DimensionRegistry for managing dimensions.
#[pyclass(name = "DimensionRegistry")]
pub struct PyDimensionRegistry {
    _inner: DimensionRegistry,
}

impl From<DimensionRegistry> for PyDimensionRegistry {
    fn from(registry: DimensionRegistry) -> Self {
        PyDimensionRegistry { _inner: registry }
    }
}

#[pymethods]
impl PyDimensionRegistry {
    #[new]
    fn new() -> Self {
        DimensionRegistry::new().into()
    }

    #[getter]
    fn base_dimensions(slf: &Bound<'_, Self>) -> PyBaseDimensionsView {
        PyBaseDimensionsView {
            registry: slf.clone().unbind(),
        }
    }

    fn try_insert_new_base_dimension(
        &mut self,
        dimension: &str,
        definition: &PyBaseDimensionDef,
    ) -> PyResult<()> {
        let result = self
            ._inner
            .try_insert_new_base_dimension(dimension, definition._inner.clone());
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to insert base dimension: {:?}",
                e
            ))),
        }
    }

    fn replace_base_dimension(
        &mut self,
        dimension: &str,
        definition: &PyBaseDimensionDef,
    ) -> PyResult<Option<PyBaseDimensionDef>> {
        let previous_def = self
            ._inner
            .replace_base_dimension(dimension, definition._inner.clone());
        Ok(previous_def.map(|def| def.into()))
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

#[pyclass(mapping, name = "BaseDimensionsView")]
pub struct PyBaseDimensionsView {
    registry: Py<PyDimensionRegistry>,
}

#[pymethods]
impl PyBaseDimensionsView {
    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<PyBaseDimensionDef> {
        self.registry
            .borrow(py)
            ._inner
            .base_dimensions()
            .get(key)
            .map(|def| def.clone().into())
            .ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!("Key '{}' not found", key))
            })
    }

    fn __len__(&self, py: Python<'_>) -> usize {
        self.registry.borrow(py)._inner.base_dimensions().len()
    }

    fn __iter__<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyIterator>> {
        let py = slf.py();
        let keys: Vec<String> = {
            let this = slf.borrow();
            let registry = this.registry.borrow(py);
            registry._inner.base_dimensions().keys().cloned().collect()
        };
        let py_list = PyList::new(py, keys)?;
        PyIterator::from_object(py_list.as_any())
    }

    fn __contains__(&self, py: Python<'_>, key: &str) -> bool {
        self.registry
            .borrow(py)
            ._inner
            .base_dimensions()
            .contains_key(key)
    }

    fn keys(slf: &Bound<'_, Self>) -> PyResult<Vec<String>> {
        let iter = Self::__iter__(slf)?;
        let mut keys = Vec::new();
        for key in iter {
            keys.push(key?.extract()?);
        }
        Ok(keys)
    }

    fn values(slf: &Bound<'_, Self>) -> PyResult<Vec<PyBaseDimensionDef>> {
        let py = slf.py();
        let this = slf.borrow();
        let iter = Self::__iter__(slf)?;
        let mut values = Vec::new();
        for key in iter {
            let key_str: String = key?.extract()?;
            values.push(this.__getitem__(py, &key_str)?);
        }
        Ok(values)
    }

    fn items(slf: &Bound<'_, Self>) -> PyResult<Vec<(String, PyBaseDimensionDef)>> {
        let py = slf.py();
        let this = slf.borrow();
        let iter = Self::__iter__(slf)?;
        let mut items = Vec::new();
        for key in iter {
            let key_str: String = key?.extract()?;
            let value = this.__getitem__(py, &key_str)?;
            items.push((key_str, value));
        }
        Ok(items)
    }

    #[pyo3(signature = (key, default=None))]
    fn get(
        &self,
        py: Python<'_>,
        key: &str,
        default: Option<PyBaseDimensionDef>,
    ) -> Option<PyBaseDimensionDef> {
        match self.__getitem__(py, key) {
            Ok(def) => Some(def),
            Err(_) => default,
        }
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name: Bound<'_, PyString> = slf.get_type().qualname()?;
        Ok(format!("{}()", class_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_dimension_def_creation() {
        let dimension = PyBaseDimensionDef::new("length", "L");
        assert_eq!(dimension.name(), "length");
        assert_eq!(dimension.symbol(), "L");
    }

    #[test]
    fn test_dimension_registry_creation() {
        let _registry = PyDimensionRegistry::new();
    }

    #[test]
    fn test_try_insert_new_base_dimension() {
        let mut registry = PyDimensionRegistry::new();
        let dimension = PyBaseDimensionDef::new("length", "L");
        let result = registry.try_insert_new_base_dimension("length", &dimension);
        assert!(result.is_ok());
        assert!(registry._inner.base_dimensions().contains_key("length"));
        let result = registry.try_insert_new_base_dimension("length", &dimension);
        assert!(result.is_err());
    }

    #[test]
    fn test_replace_base_dimension() {
        let mut registry = PyDimensionRegistry::new();
        let dimension1 = PyBaseDimensionDef::new("length", "L");
        let dimension2 = PyBaseDimensionDef::new("length", "Len");
        let previous = registry
            .replace_base_dimension("length", &dimension1)
            .unwrap();
        assert!(previous.is_none());
        let previous = registry
            .replace_base_dimension("length", &dimension2)
            .unwrap();
        assert!(previous.is_some());
        assert!(previous.unwrap() == dimension1);
        let current_def = registry._inner.base_dimensions().get("length").unwrap();
        assert_eq!(current_def.symbol(), "Len");
    }
}
