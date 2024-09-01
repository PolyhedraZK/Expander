use crate::{Field, FieldSerde};

/// Configurations for the SimdField.
pub trait SimdField: From<Self::Scalar> + Field + FieldSerde {
    /// Field for the challenge. Can be self.
    type Scalar: Field + FieldSerde + Send;

    /// scale self with the challenge
    fn scale(&self, challenge: &Self::Scalar) -> Self;

    /// unpack into a vector. (Can we use array? with constant generic)
    fn unpack(&self) -> Vec<Self::Scalar>;

    fn pack_size() -> usize;
}
