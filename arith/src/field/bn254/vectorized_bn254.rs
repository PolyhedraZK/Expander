use std::{
    iter::{Product, Sum},
    mem::size_of,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use halo2curves::bn256::Fr;
use rand::RngCore;

use crate::{Field, FieldSerde, VectorizedField};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VectorizedFr {
    pub v: [Fr; 1],
}

pub const VECTORIZEDM31_INV_2: VectorizedFr = VectorizedFr { v: [Fr::INV_2; 1] };

impl FieldSerde for VectorizedFr {
    // todo: turn serialization functions into a trait
    // perhaps derive from Serde or ark-serde

    #[inline(always)]
    fn serialize_into(&self, buffer: &mut [u8]) {
        self.v[0].serialize_into(buffer);
    }

    #[inline(always)]
    fn deserialize_from(buffer: &[u8]) -> Self {
        let v = Fr::deserialize_from(buffer);
        Self { v: [v] }
    }
}

impl Field for VectorizedFr {
    const NAME: &'static str = "Vectorized Mersenne 31";

    const SIZE: usize = size_of::<[Fr; 1]>();

    const INV_2: Self = VECTORIZEDM31_INV_2;

    type BaseField = Fr;

    #[inline(always)]
    fn zero() -> Self {
        VectorizedFr { v: [Fr::zero(); 1] }
    }

    #[inline(always)]
    fn one() -> Self {
        VectorizedFr { v: [Fr::one(); 1] }
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl RngCore) -> Self {
        VectorizedFr {
            v: [Fr::random_unsafe(&mut rng)],
        }
    }

    #[inline(always)]
    fn random_bool_unsafe(mut rng: impl RngCore) -> Self {
        VectorizedFr {
            v: [Fr::random_bool_unsafe(&mut rng)],
        }
    }

    fn exp(&self, _exponent: &Self) -> Self {
        unimplemented!()
    }

    fn inv(&self) -> Option<Self> {
        unimplemented!()
    }

    #[inline(always)]
    fn add_base_elem(&self, _rhs: &Self::BaseField) -> Self {
        unimplemented!()
    }

    #[inline(always)]
    fn add_assign_base_elem(&mut self, rhs: &Self::BaseField) {
        *self += rhs;
    }

    #[inline(always)]
    fn mul_base_elem(&self, rhs: &Self::BaseField) -> Self {
        *self * rhs
    }

    #[inline(always)]
    fn mul_assign_base_elem(&mut self, rhs: &Self::BaseField) {
        *self = *self * rhs;
    }

    fn as_u32_unchecked(&self) -> u32 {
        unimplemented!("self is a vector, cannot convert to u32")
    }
    fn from_uniform_bytes(_bytes: &[u8; 32]) -> Self {
        unimplemented!(" cannot convert 32 bytes into a vectorized M31")
    }
}

impl VectorizedField for VectorizedFr {
    const PACK_SIZE: usize = Fr::SIZE;

    const VECTORIZE_SIZE: usize = 1;

    type PackedBaseField = Fr;

    #[inline(always)]
    fn as_packed_slices(&self) -> &[Fr] {
        &self.v
    }

    #[inline(always)]
    fn mut_packed_slices(&mut self) -> &mut [Self::PackedBaseField] {
        &mut self.v
    }
}

impl Mul<&VectorizedFr> for VectorizedFr {
    type Output = VectorizedFr;
    #[inline(always)]
    fn mul(self, rhs: &VectorizedFr) -> Self::Output {
        VectorizedFr {
            v: self
                .v
                .iter()
                .zip(rhs.v.iter())
                .map(|(a, b)| *a * b)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }
}

impl Mul for VectorizedFr {
    type Output = VectorizedFr;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn mul(self, rhs: VectorizedFr) -> Self::Output {
        self * &rhs
    }
}

impl Mul<&Fr> for VectorizedFr {
    type Output = VectorizedFr;
    #[inline(always)]
    fn mul(self, rhs: &Fr) -> Self::Output {
        VectorizedFr {
            v: [self.v[0] * rhs],
        }
    }
}

impl Mul<Fr> for VectorizedFr {
    type Output = VectorizedFr;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn mul(self, rhs: Fr) -> Self::Output {
        self * &rhs
    }
}

impl MulAssign<&VectorizedFr> for VectorizedFr {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: &VectorizedFr) {
        *self = *self * rhs;
    }
}

impl MulAssign for VectorizedFr {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        *self *= &rhs;
    }
}

impl<T: ::core::borrow::Borrow<VectorizedFr>> Product<T> for VectorizedFr {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Add<&VectorizedFr> for VectorizedFr {
    type Output = VectorizedFr;
    #[inline(always)]
    fn add(self, rhs: &VectorizedFr) -> Self::Output {
        VectorizedFr {
            v: self
                .v
                .iter()
                .zip(rhs.v.iter())
                .map(|(a, b)| *a + b)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }
}

impl Add for VectorizedFr {
    type Output = VectorizedFr;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn add(self, rhs: VectorizedFr) -> Self::Output {
        self + &rhs
    }
}

impl AddAssign<&VectorizedFr> for VectorizedFr {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &VectorizedFr) {
        self.v
            .iter_mut()
            .zip(rhs.v.iter())
            .for_each(|(a, b)| *a += b);
    }
}

impl AddAssign for VectorizedFr {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl<T: ::core::borrow::Borrow<VectorizedFr>> Sum<T> for VectorizedFr {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl AddAssign<&Fr> for VectorizedFr {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &Fr) {
        self.v[0] += rhs;
    }
}

impl AddAssign<Fr> for VectorizedFr {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Fr) {
        *self += &rhs;
    }
}

impl Neg for VectorizedFr {
    type Output = VectorizedFr;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        VectorizedFr { v: [-self.v[0]] }
    }
}

impl Sub<&VectorizedFr> for VectorizedFr {
    type Output = VectorizedFr;
    #[inline(always)]
    fn sub(self, rhs: &VectorizedFr) -> Self::Output {
        VectorizedFr {
            v: self
                .v
                .iter()
                .zip(rhs.v.iter())
                .map(|(a, b)| *a - b)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }
}

impl Sub for VectorizedFr {
    type Output = VectorizedFr;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn sub(self, rhs: VectorizedFr) -> Self::Output {
        self - &rhs
    }
}

impl SubAssign<&VectorizedFr> for VectorizedFr {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: &VectorizedFr) {
        *self = *self - rhs;
    }
}

impl SubAssign for VectorizedFr {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}

impl From<u32> for VectorizedFr {
    #[inline(always)]
    fn from(x: u32) -> Self {
        VectorizedFr {
            v: [Fr::from(x); 1],
        }
    }
}
