use std::{
    io::{Read, Write},
    iter::{Product, Sum},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use crate::{Field, FieldSerde, M31Ext3, PackedM31Ext3, VectorizedField};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VectorizedM31Ext3 {
    pub v: [PackedM31Ext3; 1],
}

impl FieldSerde for VectorizedM31Ext3 {
    #[inline(always)]
    fn serialize_into<W: Write>(&self, mut writer: W) {
        self.v[0].serialize_into(&mut writer);
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
            v: [PackedM31Ext3::deserialize_from(&mut reader)],
        }
    }

    #[inline(always)]
    fn deserialize_from_ecc_format<R: Read>(mut reader: R) -> Self {
        Self {
            v: [PackedM31Ext3::deserialize_from_ecc_format(&mut reader)],
        }
    }
}

impl VectorizedField for VectorizedM31Ext3 {
    const PACK_SIZE: usize = 8;

    const VECTORIZE_SIZE: usize = 1;

    type PackedBaseField = PackedM31Ext3;

    #[inline(always)]
    fn as_packed_slices(&self) -> &[PackedM31Ext3] {
        &self.v
    }

    #[inline(always)]
    fn mut_packed_slices(&mut self) -> &mut [Self::PackedBaseField] {
        &mut self.v
    }
}

impl Field for VectorizedM31Ext3 {
    const NAME: &'static str = "AVX Vectorized Mersenne 31 Extension 3";

    const SIZE: usize = 96;

    // const INV_2: PackedM31Ext3 = PackedM31Ext3 {
    //     v: [PackedM31::INV_2, PackedM31::zero(), PackedM31::zero()],
    // };
    const INV_2: Self = todo!();

    type BaseField = PackedM31Ext3;

    #[inline(always)]
    fn zero() -> Self {
        VectorizedM31Ext3 {
            v: [PackedM31Ext3::zero()],
        }
    }

    #[inline(always)]
    fn one() -> Self {
        VectorizedM31Ext3 {
            v: [PackedM31Ext3::one()],
        }
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        VectorizedM31Ext3 {
            v: [PackedM31Ext3::random_unsafe(&mut rng)],
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        VectorizedM31Ext3 {
            v: [PackedM31Ext3::random_bool(&mut rng)],
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
        todo!()
        // *self += rhs;
    }

    #[inline(always)]
    fn mul_base_elem(&self, rhs: &Self::BaseField) -> Self {
        todo!()
        // *self * rhs
    }

    #[inline(always)]
    fn mul_assign_base_elem(&mut self, rhs: &Self::BaseField) {
        todo!()
        // *self = *self * rhs;
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
            v: [self.v[0] * rhs.v[0]],
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
            v: [self.v[0] + rhs.v[0]],
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
        VectorizedM31Ext3 { v: [-self.v[0]] }
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
            v: [PackedM31Ext3::from(x)],
        }
    }
}
