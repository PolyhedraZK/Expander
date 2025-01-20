use crate::Field;

/// Configurations for the SimdField.
pub trait SimdField: From<Self::Scalar> + Field {
    /// Field for the challenge. Can be self.
    type Scalar: Field + Send;

    /// Pack size (width) for the SIMD instruction
    const PACK_SIZE: usize;

    /// scale self with the challenge
    fn scale(&self, challenge: &Self::Scalar) -> Self;

    /// pack a vec of scalar field into self
    fn pack(base_vec: &[Self::Scalar]) -> Self;

    /// unpack into a vector.
    fn unpack(&self) -> Vec<Self::Scalar>;
}
