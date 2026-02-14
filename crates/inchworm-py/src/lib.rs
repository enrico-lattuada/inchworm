use pyo3::prelude::*;

mod dimensions;
mod errors;

/// A Python module implemented in Rust.
#[pymodule(name = "inchworm")]
mod inchworm_py {

    #[pymodule_export]
    use super::dimensions_py;
}

#[pymodule(name = "dimensions")]
mod dimensions_py {
    #[pymodule_export]
    use super::dimensions::PyBaseDimensionDef;
    #[pymodule_export]
    use super::dimensions::PyDerivedDimensionDef;
    #[pymodule_export]
    use super::dimensions::PyDimensionRegistry;
}
