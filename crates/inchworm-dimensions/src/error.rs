use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error(
        "Cannot register base dimension: a base dimension '{dimension}' already exists in the registry"
    )]
    BaseDimensionAlreadyDefined { dimension: String },
}
