use thiserror::Error;

use crate::RegistryId;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DimensionError {
    #[error("exponent arithmetic overflow")]
    ExponentOverflow,

    #[error("zero denominator in exponent")]
    ZeroDenominator,

    #[error("cannot mix dimensions from registry `{left:?}` and registry `{right:?}`")]
    CrossRegistry { left: RegistryId, right: RegistryId },
}
