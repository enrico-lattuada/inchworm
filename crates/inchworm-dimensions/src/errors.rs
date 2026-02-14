use thiserror::Error;

#[derive(Debug, Error)]
pub enum DimensionError {
    #[error("Invalid dimension definition: {0}.")]
    InvalidDefinition(String),
}
