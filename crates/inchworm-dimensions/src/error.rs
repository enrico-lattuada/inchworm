use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DimensionError {
    #[error("exponent arithmetic overflow")]
    ExponentOverflow,

    #[error("zero denominator in exponent")]
    ZeroDenominator,
}
