use inchworm::dimensions::DimensionError;
use pyo3::{PyErr, exceptions::PyValueError};

/// Converts a [`DimensionError`] into a [`PyErr`] that can be raised in Python.
pub fn dimension_error_to_pyerr(err: DimensionError) -> PyErr {
    match err {
        DimensionError::InvalidDefinition(msg) => PyValueError::new_err(msg),
        DimensionError::InvalidComponent(msg) => PyValueError::new_err(msg),
    }
}
