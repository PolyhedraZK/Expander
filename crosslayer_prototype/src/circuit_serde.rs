use gkr_field_config::GKRFieldConfig;
use thiserror::Error;

use arith::FieldSerdeError;
use crate::*;

#[derive(Debug, Error)]
pub enum CircuitError {
    #[error("field serde error: {0:?}")]
    FieldSerdeError(#[from] FieldSerdeError),

    #[error("other error: {0:?}")]
    OtherError(#[from] std::io::Error),
}

