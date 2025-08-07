use thiserror::Error;

#[derive(Error, Debug)]
pub enum SerdeError {
    #[error("IO Error: {0}")]
    IOError(#[from] ark_std::io::Error),

    #[error("Deserialization failure")]
    DeserializeError,

    #[error("Invalid variant index: {0}")]
    InvalidVariantIndex(usize),
}

pub type SerdeResult<T> = ark_std::result::Result<T, SerdeError>;
