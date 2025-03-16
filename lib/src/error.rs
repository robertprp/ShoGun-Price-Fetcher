use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unknown error")]
    Unknown,

    #[error("Failed to serialize")]
    Serialization,

    #[error("Failed to deserialize")]
    Deserialization,

    #[error("Invalid config")]
    InvalidConfig,

    #[error("Failed to fetch")]
    FetchError,
}
