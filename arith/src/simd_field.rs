use crate::{Field, FieldSerde};

/// Configurations for the SimdField.
pub trait SimdField: From<Self::Scalar> + Field + FieldSerde {
    /// Field for the challenge. Can be self.
    type Scalar: Field + FieldSerde + Send;

    /// scale self with the challenge
    fn scale(&self, challenge: &Self::Scalar) -> Self;
}
