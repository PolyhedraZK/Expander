use halo2curves::bn256::Fr;
use rand::RngCore;
use std::{
    io::{Read, Write},
    iter::{Product, Sum},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use super::BinomialExtensionField;
use crate::{Field, FieldSerde, SimdField};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Bn254DummyExt3 {
    pub v: Fr,
}

impl SimdField for Bn254DummyExt3 {
    type Scalar = Bn254DummyExt3;

    #[inline]
    fn scale(&self, challenge: &Self::Scalar) -> Self {
        *self * *challenge
    }
}

impl FieldSerde for Bn254DummyExt3 {
    #[inline(always)]
    fn serialize_into<W: Write>(&self, writer: W) {
        self.v.serialize_into(writer);
    }

    #[inline(always)]
    fn serialized_size() -> usize {
        Fr::serialized_size()
    }

    // FIXME: this deserialization function auto corrects invalid inputs.
    // We should use separate APIs for this and for the actual deserialization.
    #[inline(always)]
    fn deserialize_from<R: Read>(mut reader: R) -> Self {
        Bn254DummyExt3 {
            v: Fr::deserialize_from(&mut reader),
        }
    }

    #[inline(always)]
    fn deserialize_from_ecc_format<R: Read>(mut reader: R) -> Self {
        Bn254DummyExt3 {
            v: Fr::deserialize_from_ecc_format(&mut reader),
        }
    }
}

impl Field for Bn254DummyExt3 {
    const NAME: &'static str = "Bn254 Dummy Extension";

    const SIZE: usize = Fr::SIZE;

    const INV_2: Bn254DummyExt3 = Bn254DummyExt3 { v: Fr::INV_2 };

    const ZERO: Bn254DummyExt3 = Bn254DummyExt3 { v: Fr::ZERO };

    #[inline(always)]
    fn zero() -> Self {
        Bn254DummyExt3 { v: Fr::zero() }
    }

    #[inline(always)]
    fn one() -> Self {
        Bn254DummyExt3 { v: Fr::one() }
    }

    fn random_unsafe(mut rng: impl RngCore) -> Self {
        Bn254DummyExt3 {
            v: Fr::random_unsafe(&mut rng),
        }
    }

    fn random_bool(mut rng: impl RngCore) -> Self {
        Bn254DummyExt3 {
            v: Fr::random_bool(&mut rng),
        }
    }

    fn exp(&self, exponent: &Self) -> Self {
        Bn254DummyExt3 {
            v: self.v.exp(&exponent.v),
        }
    }

    fn inv(&self) -> Option<Self> {
        self.v.inv().map(|v| Bn254DummyExt3 { v })
    }

    /// Squaring
    #[inline(always)]
    fn square(&self) -> Self {
        Bn254DummyExt3 { v: self.v.square() }
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        self.v.as_u32_unchecked()
    }

    #[inline(always)]
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        Bn254DummyExt3 {
            v: Fr::from_uniform_bytes(bytes),
        }
    }
}

impl BinomialExtensionField<3> for Bn254DummyExt3 {
    /// Extension Field
    const W: u32 = 0; // not valid for Bn254DummyExt3

    /// Base field for the extension
    type BaseField = Fr;

    /// Multiply the extension field with the base field
    fn mul_by_base_field(&self, base: &Self::BaseField) -> Self {
        Bn254DummyExt3 { v: self.v * base }
    }

    /// Add the extension field with the base field
    fn add_by_base_field(&self, base: &Self::BaseField) -> Self {
        Bn254DummyExt3 { v: self.v + base }
    }
}

// ====================================
// Arithmetics for M31Ext
// ====================================

impl Mul<&Bn254DummyExt3> for Bn254DummyExt3 {
    type Output = Bn254DummyExt3;
    #[inline(always)]
    fn mul(self, rhs: &Bn254DummyExt3) -> Self::Output {
        Bn254DummyExt3 { v: self.v * rhs.v }
    }
}

impl Mul for Bn254DummyExt3 {
    type Output = Bn254DummyExt3;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn mul(self, rhs: Bn254DummyExt3) -> Self::Output {
        self * &rhs
    }
}

impl MulAssign<&Bn254DummyExt3> for Bn254DummyExt3 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: &Bn254DummyExt3) {
        *self = *self * rhs;
    }
}

impl MulAssign for Bn254DummyExt3 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        *self *= &rhs;
    }
}

impl<T: ::core::borrow::Borrow<Bn254DummyExt3>> Product<T> for Bn254DummyExt3 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Add<&Bn254DummyExt3> for Bn254DummyExt3 {
    type Output = Bn254DummyExt3;
    #[inline(always)]
    fn add(self, rhs: &Bn254DummyExt3) -> Self::Output {
        Bn254DummyExt3 { v: self.v + rhs.v }
    }
}

impl Add for Bn254DummyExt3 {
    type Output = Bn254DummyExt3;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn add(self, rhs: Bn254DummyExt3) -> Self::Output {
        self + &rhs
    }
}

impl AddAssign<&Bn254DummyExt3> for Bn254DummyExt3 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &Bn254DummyExt3) {
        *self = *self + rhs;
    }
}

impl AddAssign for Bn254DummyExt3 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl<T: ::core::borrow::Borrow<Bn254DummyExt3>> Sum<T> for Bn254DummyExt3 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl Neg for Bn254DummyExt3 {
    type Output = Bn254DummyExt3;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        Bn254DummyExt3 { v: -self.v }
    }
}

impl Sub<&Bn254DummyExt3> for Bn254DummyExt3 {
    type Output = Bn254DummyExt3;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn sub(self, rhs: &Bn254DummyExt3) -> Self::Output {
        self + &(-*rhs)
    }
}

impl Sub for Bn254DummyExt3 {
    type Output = Bn254DummyExt3;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn sub(self, rhs: Bn254DummyExt3) -> Self::Output {
        self - &rhs
    }
}

impl SubAssign<&Bn254DummyExt3> for Bn254DummyExt3 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: &Bn254DummyExt3) {
        *self = *self - rhs;
    }
}

impl SubAssign for Bn254DummyExt3 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}

impl From<u32> for Bn254DummyExt3 {
    #[inline(always)]
    fn from(x: u32) -> Self {
        Bn254DummyExt3 { v: Fr::from(x) }
    }
}

impl Bn254DummyExt3 {
    #[inline(always)]
    pub fn to_base_field(&self) -> Fr {
        self.to_base_field_unsafe()
    }

    #[inline(always)]
    pub fn to_base_field_unsafe(&self) -> Fr {
        self.v
    }
}

impl From<Fr> for Bn254DummyExt3 {
    #[inline(always)]
    fn from(x: Fr) -> Self {
        Bn254DummyExt3 { v: x }
    }
}

impl From<&Fr> for Bn254DummyExt3 {
    #[inline(always)]
    fn from(x: &Fr) -> Self {
        Bn254DummyExt3 { v: *x }
    }
}

impl From<Bn254DummyExt3> for Fr {
    #[inline(always)]
    fn from(x: Bn254DummyExt3) -> Self {
        x.to_base_field()
    }
}

impl From<&Bn254DummyExt3> for Fr {
    #[inline(always)]
    fn from(x: &Bn254DummyExt3) -> Self {
        x.to_base_field()
    }
}
