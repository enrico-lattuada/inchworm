use thiserror::Error;

use crate::RegistryId;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DimensionError {
    #[error("dimension name `{name}` is already defined in registry `{registry}`")]
    DuplicateName { name: String, registry: String },

    #[error("cannot remove `{name}`: referenced by {dependents:?}")]
    RemovalBlocked {
        name: String,
        dependents: Vec<String>,
    },

    #[error("exponent arithmetic overflow")]
    ExponentOverflow,

    #[error("zero denominator in exponent")]
    ZeroDenominator,

    #[error("cannot mix dimensions from registry `{left:?}` and registry `{right:?}`")]
    CrossRegistry { left: RegistryId, right: RegistryId },
}
