use inchworm::dimensions::{BaseDimensionDef, DimensionRegistry};
use pyo3::prelude::*;
use pyo3::types::PyString;

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

    /// Returns a string representation of the base dimension definition.
    fn __str__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let this = slf.borrow();
        let name = this.name();
        let symbol = this.symbol();
        Ok(format!("{} ([{}])", name, symbol))
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

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name: Bound<'_, PyString> = slf.get_type().qualname()?;
        Ok(format!("{}()", class_name))
    }

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
