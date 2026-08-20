//! The crate's unified error type.
//!
//! [`UnitError`] covers every fallible operation in the crate: registry
//! mutation, expression parsing, exponent arithmetic, TOML loading.

use thiserror::Error;

/// Everything that can go wrong when building, parsing, or combining units.
///
/// Implements [`std::error::Error`] via `thiserror`. `#[non_exhaustive]`: new
/// variants may be added without a breaking change, so external `match`es must
/// include a wildcard arm.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum UnitError {
    #[error(transparent)]
    Dimension(#[from] inchworm_dimensions::DimensionError),
}
