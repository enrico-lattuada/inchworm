use thiserror::Error;

use crate::RegistryId;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DimensionError {
    #[error("dimension name `{name}` is already defined in registry `{registry}`")]
    DuplicateName { name: String, registry: String },

    #[error("unknown dimension `{name}` in registry `{registry}`")]
    UnknownDimension { name: String, registry: String },

    #[error("cannot mix dimensions from registry `{left:?}` and registry `{right:?}`")]
    CrossRegistry { left: RegistryId, right: RegistryId },

    #[error("cannot remove `{name}`: referenced by {dependents:?}")]
    RemovalBlocked {
        name: String,
        dependents: Vec<String>,
    },

    #[error("cyclic definition(s) involving {names:?}")]
    CyclicDefinition { names: Vec<String> },

    #[error("`{name}` is declared dimensionless but reduces to signature `{signature}`")]
    NotDimensionless { name: String, signature: String },

    #[error("parse error at byte {offset}: {message} in `{src}`")]
    Parse {
        src: String,
        offset: usize,
        message: String,
    },

    #[error("exponent arithmetic overflow")]
    ExponentOverflow,

    #[error("zero denominator in exponent")]
    ZeroDenominator,

    #[cfg(feature = "toml")]
    #[error("invalid definition file: {0}")]
    DefinitionFile(String),

    #[error("Buckingham pi requires at least one variable")]
    EmptyPiInput,
}
