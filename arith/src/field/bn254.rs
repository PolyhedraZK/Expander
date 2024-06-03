use std::mem::size_of;

use halo2curves::ff::{Field as Halo2Field, FromUniformBytes};
use halo2curves::{bn256::Fr, ff::PrimeField};
use rand::RngCore;

use crate::{Field, FieldSerde};

mod vectorized_bn254;

pub use vectorized_bn254::VectorizedFr;

impl Field for Fr {
    /// name
    const NAME: &'static str = "bn254 scalar field";

    /// size required to store the data
    const SIZE: usize = size_of::<Fr>();

    /// Inverse of 2
    const INV_2: Self = Fr::TWO_INV;

    /// type of the base field, can be itself
    type BaseField = Self;

    // ====================================
    // constants
    // ====================================
    /// Zero element
    fn zero() -> Self {
        Fr::zero()
    }

    /// Identity element
    fn one() -> Self {
        Fr::one()
    }

    // ====================================
    // generators
    // ====================================
    /// create a random element from rng.
    /// test only -- the output may not be uniformly random.
    fn random_unsafe(rng: impl RngCore) -> Self {
        Fr::random(rng)
    }

    /// create a random boolean element from rng
    fn random_bool_unsafe(mut rng: impl RngCore) -> Self {
        Self::from((rng.next_u32() & 1) as u64)
    }

    // ====================================
    // arithmetics
    // ====================================
    /// Squaring
    fn square(&self) -> Self {
        *self * *self
    }

    /// Doubling
    fn double(&self) -> Self {
        *self + *self
    }

    /// Exp
    fn exp(&self, _exponent: &Self) -> Self {
        unimplemented!()
    }

    /// find the inverse of the element; return None if not exist
    fn inv(&self) -> Option<Self> {
        self.invert().into()
    }

    /// Add the field element with its base field element
    fn add_base_elem(&self, rhs: &Self::BaseField) -> Self {
        self + rhs
    }

    /// Add the field element with its base field element
    fn add_assign_base_elem(&mut self, rhs: &Self::BaseField) {
        *self += rhs
    }

    /// multiply the field element with its base field element
    fn mul_base_elem(&self, rhs: &Self::BaseField) -> Self {
        self * rhs
    }

    /// multiply the field element with its base field element
    fn mul_assign_base_elem(&mut self, rhs: &Self::BaseField) {
        *self *= rhs
    }

    /// expose the element as u32.
    fn as_u32_unchecked(&self) -> u32 {
        todo!()
    }

    // TODO: better implementation
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        <Fr as FromUniformBytes<64>>::from_uniform_bytes(
            &[bytes.as_slice(), [0u8; 32].as_slice()]
                .concat()
                .try_into()
                .unwrap(),
        )
    }
}

impl FieldSerde for Fr {
    fn serialize_into(&self, buffer: &mut [u8]) {
        buffer.copy_from_slice(self.to_bytes().as_slice())
    }

    fn deserialize_from(buffer: &[u8]) -> Self {
        Fr::from_bytes(buffer[..Fr::SIZE].try_into().unwrap()).unwrap()
    }
}
