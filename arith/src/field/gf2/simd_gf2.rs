use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::{Field, FieldSerde, SimdField};

use super::GF2;

/// A Simdgf2 stores 512 bits of data.

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SimdGF2 {
    pub v: u8,
}

impl FieldSerde for SimdGF2 {
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
        SimdGF2 { v: u[0] }
    }

    #[inline(always)]
    fn deserialize_from_ecc_format<R: std::io::Read>(mut _reader: R) -> Self {
        let mut u = [0u8; 1];
        _reader.read_exact(&mut u).unwrap(); // todo: error propagation
        SimdGF2 { v: u[0] }
    }
}

impl Field for SimdGF2 {
    // still will pack 8 bits into a u8
    const NAME: &'static str = "Galios Field 2 SIMD";
    const SIZE: usize = 1;
    const FIELD_SIZE: usize = 1; // in bits
    const ZERO: Self = SimdGF2 { v: 0 };
    const INV_2: Self = SimdGF2 { v: 0 }; // should not be used
    #[inline(always)]
    fn zero() -> Self {
        SimdGF2 { v: 0 }
    }

    #[inline(always)]
    fn one() -> Self {
        SimdGF2 { v: 255 }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.v == 0
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        SimdGF2 {
            v: (rng.next_u32() % 256) as u8,
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        SimdGF2 {
            v: (rng.next_u32() % 256) as u8,
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
        unimplemented!()
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        self.v as u32 % 256
    }

    #[inline(always)]
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        SimdGF2 { v: bytes[0] & 255}
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

impl Mul<&SimdGF2> for SimdGF2 {
    type Output = SimdGF2;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: &SimdGF2) -> SimdGF2 {
        SimdGF2 { v: self.v & rhs.v }
    }
}

impl Mul<SimdGF2> for SimdGF2 {
    type Output = SimdGF2;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: SimdGF2) -> SimdGF2 {
        SimdGF2 { v: self.v & rhs.v }
    }
}

impl MulAssign<&SimdGF2> for SimdGF2 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn mul_assign(&mut self, rhs: &SimdGF2) {
        self.v &= rhs.v;
    }
}

impl MulAssign<SimdGF2> for SimdGF2 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn mul_assign(&mut self, rhs: SimdGF2) {
        self.v &= rhs.v;
    }
}

impl Sub for SimdGF2 {
    type Output = SimdGF2;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: SimdGF2) -> SimdGF2 {
        SimdGF2 { v: self.v ^ rhs.v }
    }
}

impl SubAssign for SimdGF2 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn sub_assign(&mut self, rhs: SimdGF2) {
        self.v ^= rhs.v;
    }
}

impl Add for SimdGF2 {
    type Output = SimdGF2;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: SimdGF2) -> SimdGF2 {
        SimdGF2 { v: self.v ^ rhs.v }
    }
}

impl AddAssign for SimdGF2 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: SimdGF2) {
        self.v ^= rhs.v;
    }
}

impl Add<&SimdGF2> for SimdGF2 {
    type Output = SimdGF2;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: &SimdGF2) -> SimdGF2 {
        SimdGF2 { v: self.v ^ rhs.v }
    }
}

impl Sub<&SimdGF2> for SimdGF2 {
    type Output = SimdGF2;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: &SimdGF2) -> SimdGF2 {
        SimdGF2 { v: self.v ^ rhs.v }
    }
}

impl<T: std::borrow::Borrow<SimdGF2>> std::iter::Sum<T> for SimdGF2 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl<T: std::borrow::Borrow<SimdGF2>> std::iter::Product<T> for SimdGF2 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Neg for SimdGF2 {
    type Output = SimdGF2;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn neg(self) -> SimdGF2 {
        SimdGF2 { v: self.v }
    }
}

impl AddAssign<&SimdGF2> for SimdGF2 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: &SimdGF2) {
        self.v ^= rhs.v;
    }
}

impl SubAssign<&SimdGF2> for SimdGF2 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn sub_assign(&mut self, rhs: &SimdGF2) {
        self.v ^= rhs.v;
    }
}

impl From<u32> for SimdGF2 {
    #[inline(always)]
    fn from(v: u32) -> Self {
        SimdGF2 { v: (v % 2) as u8 }
    }
}

impl From<GF2> for SimdGF2 {
    #[inline(always)]
    fn from(v: GF2) -> Self {
        SimdGF2 { v: v.v }
    }
}

impl SimdField for SimdGF2 {
    fn scale(&self, challenge: &Self::Scalar) -> Self {
        if challenge.v == 0 {
            Self::zero()
        } else {
            *self
        }
    }
    
    type Scalar = crate::GF2;
}
