//! The crate's unified error type.
//!
//! [`UnitError`] covers every fallible operation in the crate: registry
//! mutation, expression parsing, exponent arithmetic, TOML loading.

use thiserror::Error;

use crate::UnitRegistryId;

/// Everything that can go wrong when building, parsing, or combining units.
///
/// Implements [`std::error::Error`] via `thiserror`. `#[non_exhaustive]`: new
/// variants may be added without a breaking change, so external `match`es must
/// include a wildcard arm.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum UnitError {
    /// `name` (a unit or prefix name, or a symbol) is not a valid identifier
    #[error("name `{name}` is not a valid identifier")]
    InvalidName { name: String },

    /// `name` is already registered in `registry`, either as a unit name or as
    /// a prefix name.
    #[error("name `{name}` is already defined in registry `{registry}`")]
    DuplicateName { name: String, registry: String },

    /// `name` isn't a registered canonical name in `registry`.
    #[error("unknown unit `{name}` in registry `{registry}`")]
    UnknownUnit { name: String, registry: String },

    /// `name` isn't a registered prefix in `registry`.
    #[error("unknown prefix `{name}` in registry `{registry}`")]
    UnknownPrefix { name: String, registry: String },

    /// `name` cannot be combined with a prefix: its underlying unit was
    /// registered as non-prefixable (anchored/affine units, for instance, are
    /// never prefixable).
    #[error("unit `{name}` in registry `{registry}` is not prefixable")]
    NotPrefixable { name: String, registry: String },

    /// The two operands' atoms were minted by different `UnitRegistry`
    /// instances.
    #[error("cannot mix units from registry `{left:?}` and registry `{right:?}`")]
    CrossRegistry {
        left: UnitRegistryId,
        right: UnitRegistryId,
    },

    /// `name`'s scale must be positive and finite.
    #[error(
        "name `{name}`'s scale (`{scale}`) in registry `{registry}` must be positive and finite"
    )]
    InvalidScale {
        name: String,
        registry: String,
        scale: f64,
    },

    /// A unit expression failed to parse. `offset` is the byte offset
    /// into `src` where the error was detected.
    #[error("parse error at byte {offset}: {message} in `{src}`")]
    Parse {
        src: String,
        offset: usize,
        message: String,
    },

    /// The underlying dimension algebra failed: incompatible dimensions,
    /// exponent overflow, and so on.
    /// See [`DimensionError`](inchworm_dimensions::DimensionError) for detail.
    #[error(transparent)]
    Dimension(#[from] inchworm_dimensions::DimensionError),
}
