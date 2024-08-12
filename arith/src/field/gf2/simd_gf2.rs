use crate::SimdField;

/// A Simdgf2 stores 512 bits of data.
pub type SimdGF2 = super::GF2;

impl SimdField for SimdGF2 {
    fn scale(&self, challenge: &Self::Scalar) -> Self {
        *self * *challenge
    }
    type Scalar = crate::GF2;
}