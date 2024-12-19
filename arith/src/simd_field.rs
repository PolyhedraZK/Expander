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

    /// pack a vec of simd field with same scalar field into self
    fn pack_from_simd<PackF>(simd_vec: &[PackF]) -> Self
    where
        PackF: SimdField<Scalar = Self::Scalar>,
    {
        assert_eq!(simd_vec.len() * PackF::PACK_SIZE, Self::PACK_SIZE);
        let mut temp: Vec<_> = simd_vec.to_vec();
        temp.reverse();

        unsafe { *(temp.as_ptr() as *const Self) }
    }

    /// unpack into a vector.
    fn unpack(&self) -> Vec<Self::Scalar>;
}
