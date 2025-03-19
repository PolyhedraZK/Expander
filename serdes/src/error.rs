use thiserror::Error;

#[derive(Error, Debug)]
pub enum SerdeError {
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Deserialization failure")]
    DeserializeError,
}

pub type SerdeResult<T> = std::result::Result<T, SerdeError>;
