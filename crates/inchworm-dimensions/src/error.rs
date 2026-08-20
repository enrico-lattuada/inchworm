//! The crate's unified error type.
//!
//! [`DimensionError`] covers every fallible operation in the crate: registry
//! mutation, expression parsing, exponent arithmetic, TOML loading, and
//! Buckingham pi analysis.

use thiserror::Error;

use crate::RegistryId;

/// Everything that can go wrong when building, parsing, or combining dimensions.
///
/// Implements [`std::error::Error`] via `thiserror`. `#[non_exhaustive]`: new
/// variants may be added without a breaking change, so external `match`es must
/// include a wildcard arm.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DimensionError {
    /// `name` is already registered in `registry`, either as a canonical name
    /// or as an alias.
    #[error("dimension name `{name}` is already defined in registry `{registry}`")]
    DuplicateName { name: String, registry: String },

    /// `name` isn't a registered canonical name or alias in `registry`.
    #[error("unknown dimension `{name}` in registry `{registry}`")]
    UnknownDimension { name: String, registry: String },

    /// An operation mixed dimensions from two different [`DimRegistry`](crate::DimRegistry)
    /// instances, identified by `left` and `right`.
    #[error("cannot mix dimensions from registry `{left:?}` and registry `{right:?}`")]
    CrossRegistry { left: RegistryId, right: RegistryId },

    /// `name` can't be removed because `dependents` still reference it in
    /// their own definitions.
    #[error("cannot remove `{name}`: referenced by {dependents:?}")]
    RemovalBlocked {
        name: String,
        dependents: Vec<String>,
    },

    /// The definitions named in `names` form a cycle (or are blocked by one).
    #[error("cyclic definition(s) involving {names:?}")]
    CyclicDefinition { names: Vec<String> },

    /// `name` was declared dimensionless, but its definition actually reduces
    /// to the non-empty base `signature` shown.
    #[error("`{name}` is declared dimensionless but reduces to signature `{signature}`")]
    NotDimensionless { name: String, signature: String },

    /// A dimension expression failed to parse. `offset` is the byte offset
    /// into `src` where the error was detected.
    #[error("parse error at byte {offset}: {message} in `{src}`")]
    Parse {
        src: String,
        offset: usize,
        message: String,
    },

    /// Exponent arithmetic (addition, multiplication, negation, or scaling)
    /// would overflow [`i64`].
    #[error("exponent arithmetic overflow")]
    ExponentOverflow,

    /// A rational exponent was constructed or parsed with a zero denominator.
    #[error("zero denominator in exponent")]
    ZeroDenominator,

    /// A TOML definition file or string couldn't be read or deserialized;
    /// the message describes what went wrong. Only constructed when the
    /// `toml` feature is enabled.
    #[cfg(feature = "toml")]
    #[error("invalid definition file: {0}")]
    DefinitionFile(String),

    /// [`buckingham_pi`](crate::buckingham_pi) was called with no variables.
    #[error("Buckingham pi requires at least one variable")]
    EmptyPiInput,
}
