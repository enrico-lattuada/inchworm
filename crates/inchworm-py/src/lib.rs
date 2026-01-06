use inchworm;
use pyo3::prelude::*;

/// Adds two unsigned 64-bit integers.
#[pyfunction]
fn add(left: u64, right: u64) -> u64 {
    inchworm::add(left, right)
}

/// A Python module implemented in Rust.
#[pymodule(name = "inchworm")]
mod inchworm_py {

    /// Formats the sum of two numbers as string.
    #[pymodule_export]
    use super::add;
}
