use std::mem::size_of;

// use halo2curves::ff::{Field as Halo2Field, FromUniformBytes};
// use halo2curves::{bn256::Fr, ff::PrimeField};

use ff::{Field as FFField, PrimeField};
use p3_bn254_fr::{Bn254Fr as Fr, FFBn254Fr, FFBn254FrRepr};
use p3_field::AbstractField;
use p3_field::Field as P3Field;
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
    const INV_2: Self = Self {
        value: <FFBn254Fr as PrimeField>::TWO_INV,
    };

    /// type of the base field, can be itself
    type BaseField = Self;

    // ====================================
    // constants
    // ====================================
    /// Zero element
    fn zero() -> Self {
        Self {
            value: <FFBn254Fr as FFField>::ZERO,
        }
    }

    /// Identity element
    fn one() -> Self {
        Self {
            value: <FFBn254Fr as FFField>::ONE,
        }
    }

    // ====================================
    // generators
    // ====================================
    /// create a random element from rng.
    /// test only -- the output may not be uniformly random.
    fn random_unsafe(rng: impl RngCore) -> Self {
        Self {
            value: FFBn254Fr::random(rng),
        }
    }

    /// create a random boolean element from rng
    fn random_bool_unsafe(mut rng: impl RngCore) -> Self {
        Self::from_bool((rng.next_u32() & 1) != 0)
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
        self.try_inverse()
    }

    /// Add the field element with its base field element
    fn add_base_elem(&self, rhs: &Self::BaseField) -> Self {
        *self + *rhs
    }

    /// Add the field element with its base field element
    fn add_assign_base_elem(&mut self, rhs: &Self::BaseField) {
        *self += *rhs
    }

    /// multiply the field element with its base field element
    fn mul_base_elem(&self, rhs: &Self::BaseField) -> Self {
        *self * *rhs
    }

    /// multiply the field element with its base field element
    fn mul_assign_base_elem(&mut self, rhs: &Self::BaseField) {
        *self *= *rhs
    }

    fn from_u32(value: u32) -> Self {
        <Self as AbstractField>::from_canonical_u32(value as u32)
    }

    /// expose the element as u32.
    fn as_u32_unchecked(&self) -> u32 {
        todo!()
    }

    // TODO: better implementation
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        let mut tmp = bytes.clone();
        tmp[31] &= 0b0000_0111;
        Self::deserialize_from(&tmp)
    }
}

impl FieldSerde for Fr {
    fn serialize_into(&self, buffer: &mut [u8]) {
        buffer.copy_from_slice(self.value.to_repr().as_ref())
    }

    fn deserialize_from(buffer: &[u8]) -> Self {
        let mut repr = FFBn254FrRepr::default();
        repr.as_mut().copy_from_slice(buffer[..32].as_ref());

        Self {
            value: FFBn254Fr::from_repr(repr).unwrap(),
        }
        // Fr::from_bytes(buffer[..Fr::SIZE].try_into().unwrap()).unwrap()
    }
}
