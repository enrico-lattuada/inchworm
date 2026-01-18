use pyo3::prelude::*;

mod dimensions;

/// A Python module implemented in Rust.
#[pymodule(name = "inchworm")]
mod inchworm_py {

    #[pymodule_export]
    use super::dimensions_py;
}

#[pymodule(name = "dimensions")]
mod dimensions_py {
    #[pymodule_export]
    use super::dimensions::DimensionRegistry;
}
