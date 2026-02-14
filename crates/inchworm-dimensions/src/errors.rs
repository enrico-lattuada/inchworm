use thiserror::Error;

/// Errors that can occur when defining or managing dimensions.
#[derive(Debug, Error)]
pub enum DimensionError {
    /// The dimension definition is invalid.
    #[error("Invalid dimension definition: {0}.")]
    InvalidDefinition(String),
}
