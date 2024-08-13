// Galios field over 2^128
// credit to intel for the original implementation
// https://www.intel.com/content/dam/develop/external/us/en/documents/clmul-wp-rev-2-02-2014-04-20.pdf

mod gf2x8;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

pub use gf2x8::GF2x8;

use crate::FieldSerde;

use super::Field;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GF2 {
    pub v: u8,
}

impl FieldSerde for GF2 {
    #[inline(always)]
    fn serialize_into<W: std::io::Write>(&self, mut writer: W) {
        writer.write_all(self.v.to_le_bytes().as_ref()).unwrap(); // todo: error propagation
    }

    #[inline(always)]
    fn serialized_size() -> usize {
        1
    }

    #[inline(always)]
    fn deserialize_from<R: std::io::Read>(mut reader: R) -> Self {
        let mut u = [0u8; 1];
        reader.read_exact(&mut u).unwrap(); // todo: error propagation
        GF2 { v: u[0] % 2 }
    }

    #[inline(always)]
    fn deserialize_from_ecc_format<R: std::io::Read>(mut _reader: R) -> Self {
        let mut u = [0u8; 1];
        _reader.read_exact(&mut u).unwrap(); // todo: error propagation
        GF2 { v: u[0] % 2 }
    }
}

impl Field for GF2 {
    // still will pack 8 bits into a u8
    const NAME: &'static str = "Galios Field 2";
    const SIZE: usize = 1;
    const FIELD_SIZE: usize = 1; // in bits
    const ZERO: Self = GF2 { v: 0 };
    const INV_2: Self = GF2 { v: 0 }; // should not be used
    #[inline(always)]
    fn zero() -> Self {
        GF2 { v: 0 }
    }

    #[inline(always)]
    fn one() -> Self {
        GF2 { v: 1 }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.v == 0
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        GF2 {
            v: (rng.next_u32() % 2) as u8,
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        GF2 {
            v: (rng.next_u32() % 2) as u8,
        }
    }

    #[inline(always)]
    fn exp(&self, exponent: u128) -> Self {
        if exponent % 2 == 0 {
            Self::one()
        } else {
            *self
        }
    }

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        if self.v == 0 {
            None
        } else {
            Some(Self::one())
        }
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        self.v as u32 % 2
    }

    #[inline(always)]
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        GF2 { v: bytes[0] % 2 }
    }

    #[inline(always)]
    fn mul_by_5(&self) -> Self {
        *self
    }

    #[inline(always)]
    fn mul_by_6(&self) -> Self {
        Self::ZERO
    }
}

impl Mul<&GF2> for GF2 {
    type Output = GF2;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: &GF2) -> GF2 {
        GF2 { v: self.v & rhs.v }
    }
}

impl Mul<GF2> for GF2 {
    type Output = GF2;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: GF2) -> GF2 {
        GF2 { v: self.v & rhs.v }
    }
}

impl MulAssign<&GF2> for GF2 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn mul_assign(&mut self, rhs: &GF2) {
        self.v &= rhs.v;
    }
}

impl MulAssign<GF2> for GF2 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn mul_assign(&mut self, rhs: GF2) {
        self.v &= rhs.v;
    }
}

impl Sub for GF2 {
    type Output = GF2;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: GF2) -> GF2 {
        GF2 { v: self.v ^ rhs.v }
    }
}

impl SubAssign for GF2 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn sub_assign(&mut self, rhs: GF2) {
        self.v ^= rhs.v;
    }
}

impl Add for GF2 {
    type Output = GF2;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: GF2) -> GF2 {
        GF2 { v: self.v ^ rhs.v }
    }
}

impl AddAssign for GF2 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: GF2) {
        self.v ^= rhs.v;
    }
}

impl Add<&GF2> for GF2 {
    type Output = GF2;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: &GF2) -> GF2 {
        GF2 { v: self.v ^ rhs.v }
    }
}

impl Sub<&GF2> for GF2 {
    type Output = GF2;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: &GF2) -> GF2 {
        GF2 { v: self.v ^ rhs.v }
    }
}

impl<T: std::borrow::Borrow<GF2>> std::iter::Sum<T> for GF2 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl<T: std::borrow::Borrow<GF2>> std::iter::Product<T> for GF2 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Neg for GF2 {
    type Output = GF2;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn neg(self) -> GF2 {
        GF2 { v: self.v }
    }
}

impl AddAssign<&GF2> for GF2 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: &GF2) {
        self.v ^= rhs.v;
    }
}

impl SubAssign<&GF2> for GF2 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn sub_assign(&mut self, rhs: &GF2) {
        self.v ^= rhs.v;
    }
}

impl From<u32> for GF2 {
    #[inline(always)]
    fn from(v: u32) -> Self {
        GF2 { v: (v % 2) as u8 }
    }
}
