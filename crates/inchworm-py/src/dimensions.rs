use inchworm::dimensions::{BaseDimensionDef, DimensionRegistry};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyIterator, PyList, PyString};

/// A definition of a base physical dimension.
///
/// `BaseDimensionDef` represents a fundamental physical dimension, such as
/// length, mass, and time, that form the basis for derived dimensions in a
/// units system. Each base dimension has a name and an optional symbol for
/// compact representation.
///
/// Examples
/// --------
///
/// ```python
/// >>> from inchworm.dimensions import BaseDimensionDef
/// >>> length = BaseDimensionDef("length", "L")
/// ```
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
    /// Creates a new `BaseDimensionDef` with the given name and symbol.
    ///
    /// Parameters
    /// ----------
    ///
    /// * `name` - The name of the base dimension (e.g., "length", "mass").
    /// * `symbol` - A symbol for the base dimension (e.g., "L" for length).
    ///
    /// # Returns
    ///
    /// A new `BaseDimensionDef` instance.
    ///
    /// # Examples
    ///
    /// ```python
    /// >>> dim = BaseDimensionDef("mass", "M")
    /// ```
    #[new]
    fn new(name: &str, symbol: &str) -> Self {
        BaseDimensionDef::new(name, symbol).into()
    }

    /// The name of the base dimension.
    ///
    /// # Returns
    ///
    /// * `name` - The name as a string.
    ///
    /// # Examples
    ///
    /// ```python
    /// >>> dim = BaseDimensionDef("length", "L")
    /// >>> dim.name
    /// 'length'
    /// ```
    #[getter]
    fn name(&self) -> &str {
        self._inner.name()
    }

    /// The symbol of the base dimension.
    ///
    /// # Returns
    ///
    /// * `symbol` - The symbol as a string.
    ///
    /// # Examples
    ///
    /// ```python
    /// >>> dim = BaseDimensionDef("length", "L")
    /// >>> dim.symbol
    /// 'L'
    /// ```
    #[getter]
    fn symbol(&self) -> &str {
        self._inner.symbol()
    }

    /// Returns a string representation of the base dimension definition.
    ///
    /// The format is: `BaseDimensionDef(name='<name>', symbol='<symbol>')`.
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

/// A registry for managing dimensions.
///
/// `DimensionRegistry` provides a central location to define and manage
/// physical dimensions (e.g., length, mass, time) that form the foundation
/// of the units system.
///
/// # Examples
///
/// ```python
/// >>> from inchworm.dimensions import DimensionRegistry, BaseDimensionDef
/// >>> registry = DimensionRegistry()
/// >>> length = BaseDimensionDef("length", "L")
/// >>> registry.try_insert_new_base_dimension("length", length)
/// ```
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

    /// Returns a view of all registered base dimensions in the registry.
    ///
    /// The returned view provides dict-like access to the base dimensions,
    /// supporting indexing, iteration, and membership testing.
    ///
    /// # Returns
    ///
    /// A `BaseDimensionsView` providing read-only access to base dimensions.
    ///
    /// # Examples
    ///
    /// ```python
    /// >>> registry = DimensionRegistry()
    /// >>> length = BaseDimensionDef("length", "L")
    /// >>> registry.try_insert_new_base_dimension("length", length)
    /// >>> registry.base_dimensions["length"]
    /// BaseDimensionDef(name='length', symbol='L')
    /// >>> "length" in registry.base_dimensions
    /// True
    /// ```
    #[getter]
    fn base_dimensions(slf: &Bound<'_, Self>) -> PyBaseDimensionsView {
        PyBaseDimensionsView {
            registry: slf.clone().unbind(),
        }
    }

    /// Inserts a new base dimension into the registry.
    ///
    /// This method will fail if a base dimension with the same name already
    /// exists in the registry. Use `replace_base_dimension` if you want to
    /// overwrite an existing dimension.
    ///
    /// # Arguments
    ///
    /// * `dimension` - The key/name to register the dimension under.
    /// * `definition` - The `BaseDimensionDef` to register.
    ///
    /// # Raises
    ///
    /// * `ValueError` - If a base dimension with the same name already exists.
    ///
    /// # Examples
    ///
    /// ```python
    /// >>> registry = DimensionRegistry()
    /// >>> length = BaseDimensionDef("length", "L")
    /// >>> registry.try_insert_new_base_dimension("length", length)
    /// >>> # Trying to insert again will raise ValueError
    /// >>> registry.try_insert_new_base_dimension("length", length)
    /// ValueError: Failed to insert base dimension: ...
    /// ```
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
                "Failed to insert base dimension: {}",
                e
            ))),
        }
    }

    /// Replaces an existing base dimension or inserts a new one.
    ///
    /// Unlike `try_insert_new_base_dimension`, this method will not fail if
    /// a dimension with the same name already exists. Instead, it will
    /// replace the existing definition and return the previous one.
    ///
    /// # Arguments
    ///
    /// * `dimension` - The key/name to register the dimension under.
    /// * `definition` - The `BaseDimensionDef` to register.
    ///
    /// # Returns
    ///
    /// The previous `BaseDimensionDef` if one existed, otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```python
    /// >>> registry = DimensionRegistry()
    /// >>> length1 = BaseDimensionDef("length", "L")
    /// >>> length2 = BaseDimensionDef("length", "Len")
    /// >>> registry.replace_base_dimension("length", length1)  # Returns None
    /// >>> old = registry.replace_base_dimension("length", length2)
    /// >>> old.symbol
    /// 'L'
    /// ```
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

/// A read-only, dict-like view of base dimensions in a `DimensionRegistry`.
///
/// `BaseDimensionsView` provides a read-only mapping interface to the base dimensions
/// registered in a `DimensionRegistry`.
///
/// This view does not support modification. Use the `DimensionRegistry`
/// methods to add or modify dimensions.
///
/// # Examples
///
/// ```python
/// >>> registry = DimensionRegistry()
/// >>> length = BaseDimensionDef("length", "L")
/// >>> registry.try_insert_new_base_dimension("length", length)
/// >>> view = registry.base_dimensions
/// >>> view["length"]
/// BaseDimensionDef(name='length', symbol='L')
/// >>> list(view.keys())
/// ['length']
/// ```
#[pyclass(mapping, name = "BaseDimensionsView")]
pub struct PyBaseDimensionsView {
    registry: Py<PyDimensionRegistry>,
}

#[pymethods]
impl PyBaseDimensionsView {
    /// Gets a base dimension by key.
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

    /// Returns the number of base dimensions in the registry.
    fn __len__(&self, py: Python<'_>) -> usize {
        self.registry.borrow(py)._inner.base_dimensions().len()
    }

    /// Returns an iterator over the dimension keys.
    fn __iter__<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyIterator>> {
        let py = slf.py();
        // Collect keys while holding borrow, then release before creating iterator
        let keys: Vec<String> = slf
            .borrow()
            .registry
            .borrow(py)
            ._inner
            .base_dimensions()
            .keys()
            .cloned()
            .collect();
        let py_list = PyList::new(py, keys)?;
        PyIterator::from_object(py_list.as_any())
    }

    /// Checks if a key exists in the registry.
    fn __contains__(&self, py: Python<'_>, key: &str) -> bool {
        self.registry
            .borrow(py)
            ._inner
            .base_dimensions()
            .contains_key(key)
    }

    /// Returns a list of all dimension keys.
    fn keys(&self, py: Python<'_>) -> Vec<String> {
        self.registry
            .borrow(py)
            ._inner
            .base_dimensions()
            .keys()
            .cloned()
            .collect()
    }

    /// Returns a list of all dimension definitions.
    fn values(&self, py: Python<'_>) -> Vec<PyBaseDimensionDef> {
        self.registry
            .borrow(py)
            ._inner
            .base_dimensions()
            .values()
            .cloned()
            .map(|def| def.into())
            .collect()
    }

    /// Returns a list of (key, definition) pairs.
    fn items(&self, py: Python<'_>) -> Vec<(String, PyBaseDimensionDef)> {
        self.registry
            .borrow(py)
            ._inner
            .base_dimensions()
            .iter()
            .map(|(key, def)| (key.clone(), def.clone().into()))
            .collect()
    }

    /// Gets a dimension by key, returning a default if not found.
    #[pyo3(signature = (key, default=None))]
    fn get<'py>(
        &self,
        py: Python<'py>,
        key: &str,
        default: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        match self.registry.borrow(py)._inner.base_dimensions().get(key) {
            Some(def) => {
                let py_def: PyBaseDimensionDef = def.clone().into();
                Ok(Py::new(py, py_def)?.into_bound(py).into_any())
            }
            None => Ok(default.unwrap_or_else(|| py.None().into_bound(py))),
        }
    }

    /// Returns a string representation of the view.
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name: Bound<'_, PyString> = slf.get_type().qualname()?;
        Ok(format!("{}()", class_name))
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

    /// Tests inserting a new base dimension into the registry.
    ///
    /// Verifies that:
    /// - A dimension can be successfully inserted.
    /// - Attempting to insert a duplicate dimension results in an error.
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

    /// Tests replacing a base dimension in the registry.
    ///
    /// Verifies that:
    /// - Replacing a non-existent dimension returns `None`.
    /// - Replacing an existing dimension returns the previous definition.
    /// - The new definition is correctly stored in the registry.
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
