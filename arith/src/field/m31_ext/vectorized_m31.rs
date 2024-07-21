use std::{
    io::{Read, Write},
    iter::{Product, Sum},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

#[cfg(target_arch = "x86_64")]
use crate::m31_avx::{FIVE, TEN};

#[cfg(target_arch = "aarch64")]
use crate::m31_neon::{FIVE, TEN};

use crate::{FiatShamirConfig, Field, FieldSerde, M31Ext3, VectorizedM31};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VectorizedM31Ext3 {
    pub v: [VectorizedM31; 3],
}

impl FieldSerde for VectorizedM31Ext3 {
    #[inline(always)]
    fn serialize_into<W: Write>(&self, mut writer: W) {
        self.v[0].serialize_into(&mut writer);
        self.v[1].serialize_into(&mut writer);
        self.v[2].serialize_into(&mut writer);
    }

    #[inline(always)]
    fn serialized_size() -> usize {
        96
    }

    // FIXME: this deserialization function auto corrects invalid inputs.
    // We should use separate APIs for this and for the actual deserialization.
    #[inline(always)]
    fn deserialize_from<R: Read>(mut reader: R) -> Self {
        Self {
            v: [
                VectorizedM31::deserialize_from(&mut reader),
                VectorizedM31::deserialize_from(&mut reader),
                VectorizedM31::deserialize_from(&mut reader),
            ],
        }
    }

    #[inline(always)]
    fn deserialize_from_ecc_format<R: Read>(mut reader: R) -> Self {
        Self {
            v: [
                VectorizedM31::deserialize_from_ecc_format(&mut reader),
                VectorizedM31::zero(),
                VectorizedM31::zero(),
            ],
        }
    }
}

impl FiatShamirConfig for VectorizedM31Ext3 {
    type ChallengeField = M31Ext3;

    #[inline]
    fn scale(&self, challenge: &Self::ChallengeField) -> Self {
        *self * *challenge
    }
}

impl Mul<M31Ext3> for VectorizedM31Ext3 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: M31Ext3) -> Self::Output {
        VectorizedM31Ext3 {
            v: [
                self.v[0] * rhs.v[0],
                self.v[1] * rhs.v[1],
                self.v[2] * rhs.v[2],
            ],
        }
    }
}

impl From<M31Ext3> for VectorizedM31Ext3 {
    #[inline(always)]
    fn from(x: M31Ext3) -> Self {
        Self {
            v: [
                VectorizedM31::pack_full(x.v[0]),
                VectorizedM31::pack_full(x.v[1]),
                VectorizedM31::pack_full(x.v[2]),
            ],
        }
    }
}

impl Field for VectorizedM31Ext3 {
    const NAME: &'static str = "AVX Vectorized Mersenne 31 Extension 3";

    const SIZE: usize = 96;

    const INV_2: Self = unimplemented!();

    #[inline(always)]
    fn zero() -> Self {
        VectorizedM31Ext3 {
            v: [VectorizedM31::zero(); 3],
        }
    }

    #[inline(always)]
    fn one() -> Self {
        VectorizedM31Ext3 {
            v: [
                VectorizedM31::one(),
                VectorizedM31::zero(),
                VectorizedM31::zero(),
            ],
        }
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        VectorizedM31Ext3 {
            v: [
                VectorizedM31::random_unsafe(&mut rng),
                VectorizedM31::random_unsafe(&mut rng),
                VectorizedM31::random_unsafe(&mut rng),
            ],
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        VectorizedM31Ext3 {
            v: [
                VectorizedM31::random_bool(&mut rng),
                VectorizedM31::zero(),
                VectorizedM31::zero(),
            ],
        }
    }

    #[inline(always)]
    fn square(&self) -> Self {
        Self {
            v: square_internal(&self.v),
        }
    }

    fn exp(&self, _exponent: &Self) -> Self {
        unimplemented!()
    }

    fn inv(&self) -> Option<Self> {
        unimplemented!()
    }

    fn as_u32_unchecked(&self) -> u32 {
        unimplemented!("self is a vector, cannot convert to u32")
    }

    fn from_uniform_bytes(_bytes: &[u8; 32]) -> Self {
        unimplemented!("vec m31: cannot convert from 32 bytes")
    }
}

// ====================================
// Arithmetics for M31Ext
// ====================================

impl Mul<&VectorizedM31Ext3> for VectorizedM31Ext3 {
    type Output = VectorizedM31Ext3;
    #[inline(always)]
    fn mul(self, rhs: &VectorizedM31Ext3) -> Self::Output {
        VectorizedM31Ext3 {
            v: mul_internal(&self.v, &rhs.v),
        }
    }
}

impl Mul for VectorizedM31Ext3 {
    type Output = VectorizedM31Ext3;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn mul(self, rhs: VectorizedM31Ext3) -> Self::Output {
        self * &rhs
    }
}

impl MulAssign<&VectorizedM31Ext3> for VectorizedM31Ext3 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: &VectorizedM31Ext3) {
        *self = *self * rhs;
    }
}

impl MulAssign for VectorizedM31Ext3 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        *self *= &rhs;
    }
}

impl<T: ::core::borrow::Borrow<VectorizedM31Ext3>> Product<T> for VectorizedM31Ext3 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Add<&VectorizedM31Ext3> for VectorizedM31Ext3 {
    type Output = VectorizedM31Ext3;
    #[inline(always)]
    fn add(self, rhs: &VectorizedM31Ext3) -> Self::Output {
        VectorizedM31Ext3 {
            v: [
                self.v[0] + rhs.v[0],
                self.v[1] + rhs.v[1],
                self.v[2] + rhs.v[2],
            ],
        }
    }
}

impl Add for VectorizedM31Ext3 {
    type Output = VectorizedM31Ext3;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn add(self, rhs: VectorizedM31Ext3) -> Self::Output {
        self + &rhs
    }
}

impl AddAssign<&VectorizedM31Ext3> for VectorizedM31Ext3 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &VectorizedM31Ext3) {
        *self = *self + rhs;
    }
}

impl AddAssign for VectorizedM31Ext3 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl<T: ::core::borrow::Borrow<VectorizedM31Ext3>> Sum<T> for VectorizedM31Ext3 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl Neg for VectorizedM31Ext3 {
    type Output = VectorizedM31Ext3;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        VectorizedM31Ext3 {
            v: [-self.v[0], -self.v[1], -self.v[2]],
        }
    }
}

impl Sub<&VectorizedM31Ext3> for VectorizedM31Ext3 {
    type Output = VectorizedM31Ext3;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn sub(self, rhs: &VectorizedM31Ext3) -> Self::Output {
        self + &(-*rhs)
    }
}

impl Sub for VectorizedM31Ext3 {
    type Output = VectorizedM31Ext3;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn sub(self, rhs: VectorizedM31Ext3) -> Self::Output {
        self - &rhs
    }
}

impl SubAssign<&VectorizedM31Ext3> for VectorizedM31Ext3 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: &VectorizedM31Ext3) {
        *self = *self - rhs;
    }
}

impl SubAssign for VectorizedM31Ext3 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}

impl From<u32> for VectorizedM31Ext3 {
    #[inline(always)]
    fn from(x: u32) -> Self {
        VectorizedM31Ext3 {
            v: [
                VectorizedM31::from(x),
                VectorizedM31::zero(),
                VectorizedM31::zero(),
            ],
        }
    }
}

// polynomial mod (x^3 - 5)
//
//   (a0 + a1*x + a2*x^2) * (b0 + b1*x + b2*x^2) mod (x^3 - 5)
// = a0*b0 + (a0*b1 + a1*b0)*x + (a0*b2 + a1*b1 + a2*b0)*x^2
// + (a1*b2 + a2*b1)*x^3 + a2*b2*x^4 mod (x^3 - 5)
// = a0*b0 + 5*(a1*b2 + a2*b1)
// + (a0*b1 + a1*b0)*x + 5* a2*b2
// + (a0*b2 + a1*b1 + a2*b0)*x^2
fn mul_internal(a: &[VectorizedM31; 3], b: &[VectorizedM31; 3]) -> [VectorizedM31; 3] {
    let mut res = [VectorizedM31::default(); 3];
    res[0] = a[0] * b[0] + FIVE * (a[1] * b[2] + a[2] * b[1]);
    res[1] = a[0] * b[1] + a[1] * b[0] + FIVE * a[2] * b[2];
    res[2] = a[0] * b[2] + a[1] * b[1] + a[2] * b[0];
    res
}

// same as mul; merge identical terms
fn square_internal(a: &[VectorizedM31; 3]) -> [VectorizedM31; 3] {
    let mut res = [VectorizedM31::default(); 3];
    res[0] = a[0].square() + TEN * a[1] * a[2];
    let t = a[0] * a[1];
    res[1] = t + t + FIVE * a[2].square();
    let t = a[0] * a[2];
    res[2] = t + t + a[1] * a[1];
    res
}
