//! The crate's unified error type.
//!
//! [`UnitError`] covers every fallible operation in the crate: registry
//! mutation, expression parsing, exponent arithmetic, TOML loading.

use thiserror::Error;

use crate::atom::UnitRegistryId;

/// Everything that can go wrong when building, parsing, or combining units.
///
/// Implements [`std::error::Error`] via `thiserror`. `#[non_exhaustive]`: new
/// variants may be added without a breaking change, so external `match`es must
/// include a wildcard arm.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum UnitError {
    /// `name` is already registered in `registry`.
    #[error("dimension name `{name}` is already defined in registry `{registry}`")]
    DuplicateName { name: String, registry: String },

    /// The two operands' atoms were minted by different `UnitRegistry`
    /// instances.
    #[error("cannot mix units from registry `{left:?}` and registry `{right:?}`")]
    CrossRegistry {
        left: UnitRegistryId,
        right: UnitRegistryId,
    },

    /// The underlying dimension algebra failed: incompatible dimensions,
    /// exponent overflow, and so on.
    /// See [`DimensionError`](inchworm_dimensions::DimensionError) for detail.
    #[error(transparent)]
    Dimension(#[from] inchworm_dimensions::DimensionError),
}
