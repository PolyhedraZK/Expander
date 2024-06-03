use rand::RngCore;

#[cfg(target_arch = "x86_64")]
use super::m31_avx::{PackedM31, M31_VECTORIZE_SIZE, PACKED_INV_2};
#[cfg(target_arch = "aarch64")]
use super::m31_neon::{PackedM31, M31_VECTORIZE_SIZE, PACKED_INV_2};

use crate::{Field, FieldSerde, VectorizedField, M31};
use std::{
    iter::{Product, Sum},
    mem::size_of,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

/// A VectorizedM31 stores 256 bits of data.
/// With AVX it stores a single __m256i element.
/// With NEON it stores two uint32x4_t elements.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VectorizedM31 {
    pub v: [PackedM31; VectorizedM31::VECTORIZE_SIZE],
}

pub const VECTORIZEDM31_INV_2: VectorizedM31 = VectorizedM31 {
    v: [PackedM31 { v: PACKED_INV_2 }; VectorizedM31::VECTORIZE_SIZE],
};

impl FieldSerde for VectorizedM31 {
    // todo: turn serialization functions into a trait
    // perhaps derive from Serde or ark-serde

    #[inline(always)]
    fn serialize_into(&self, buffer: &mut [u8]) {
        buffer.copy_from_slice(unsafe {
            std::slice::from_raw_parts(
                self.v.as_ptr() as *const u8,
                Self::VECTORIZE_SIZE * PackedM31::SIZE,
            )
        });
    }

    #[inline(always)]
    fn deserialize_from(buffer: &[u8]) -> Self {
        let ptr = buffer.as_ptr() as *const [PackedM31; Self::VECTORIZE_SIZE];
        unsafe {
            VectorizedM31 {
                v: ptr.read_unaligned(),
            }
        }
    }
}

impl Field for VectorizedM31 {
    const NAME: &'static str = "Vectorized Mersenne 31";

    const SIZE: usize = size_of::<[PackedM31; Self::VECTORIZE_SIZE]>();

    const INV_2: Self = VECTORIZEDM31_INV_2;

    type BaseField = M31;

    #[inline(always)]
    fn zero() -> Self {
        VectorizedM31 {
            v: [PackedM31::zero(); Self::VECTORIZE_SIZE],
        }
    }

    #[inline(always)]
    fn one() -> Self {
        VectorizedM31 {
            v: [PackedM31::one(); Self::VECTORIZE_SIZE],
        }
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl RngCore) -> Self {
        VectorizedM31 {
            v: (0..Self::VECTORIZE_SIZE)
                .map(|_| PackedM31::random_unsafe(&mut rng))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }

    #[inline(always)]
    fn random_bool_unsafe(mut rng: impl RngCore) -> Self {
        VectorizedM31 {
            v: (0..Self::VECTORIZE_SIZE)
                .map(|_| PackedM31::random_bool_unsafe(&mut rng))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
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

impl VectorizedField for VectorizedM31 {
    const VECTORIZE_SIZE: usize = M31_VECTORIZE_SIZE;
    type PackedBaseField = PackedM31;

    #[inline(always)]
    fn as_packed_slices(&self) -> &[PackedM31] {
        &self.v
    }

    #[inline(always)]
    fn mut_packed_slices(&mut self) -> &mut [Self::PackedBaseField] {
        &mut self.v
    }
}

impl Mul<&VectorizedM31> for VectorizedM31 {
    type Output = VectorizedM31;
    #[inline(always)]
    fn mul(self, rhs: &VectorizedM31) -> Self::Output {
        VectorizedM31 {
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

impl Mul for VectorizedM31 {
    type Output = VectorizedM31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn mul(self, rhs: VectorizedM31) -> Self::Output {
        self * &rhs
    }
}

impl Mul<&M31> for VectorizedM31 {
    type Output = VectorizedM31;
    #[inline(always)]
    fn mul(self, rhs: &M31) -> Self::Output {
        let mut v = [PackedM31::zero(); Self::VECTORIZE_SIZE];
        let packed_rhs = PackedM31::pack_full(*rhs);
        v.iter_mut()
            .zip(self.v.iter())
            .for_each(|(v, sv)| *v = *sv * packed_rhs);

        VectorizedM31 { v }
    }
}

impl Mul<M31> for VectorizedM31 {
    type Output = VectorizedM31;
    #[inline(always)]
    fn mul(self, rhs: M31) -> Self::Output {
        self * &rhs
    }
}

impl MulAssign<&VectorizedM31> for VectorizedM31 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: &VectorizedM31) {
        *self = *self * rhs;
    }
}

impl MulAssign for VectorizedM31 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        *self *= &rhs;
    }
}

impl<T: ::core::borrow::Borrow<VectorizedM31>> Product<T> for VectorizedM31 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Add<&VectorizedM31> for VectorizedM31 {
    type Output = VectorizedM31;
    #[inline(always)]
    fn add(self, rhs: &VectorizedM31) -> Self::Output {
        VectorizedM31 {
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

impl Add for VectorizedM31 {
    type Output = VectorizedM31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn add(self, rhs: VectorizedM31) -> Self::Output {
        self + &rhs
    }
}

impl AddAssign<&VectorizedM31> for VectorizedM31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &VectorizedM31) {
        self.v
            .iter_mut()
            .zip(rhs.v.iter())
            .for_each(|(a, b)| *a += b);
    }
}

impl AddAssign for VectorizedM31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl<T: ::core::borrow::Borrow<VectorizedM31>> Sum<T> for VectorizedM31 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl AddAssign<&M31> for VectorizedM31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &M31) {
        self.v
            .iter_mut()
            .for_each(|x| *x += PackedM31::pack_full(*rhs));
    }
}

impl AddAssign<M31> for VectorizedM31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: M31) {
        *self += &rhs;
    }
}

impl Neg for VectorizedM31 {
    type Output = VectorizedM31;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        VectorizedM31 {
            v: self
                .v
                .iter()
                .map(|a| -*a)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }
}

impl Sub<&VectorizedM31> for VectorizedM31 {
    type Output = VectorizedM31;
    #[inline(always)]
    fn sub(self, rhs: &VectorizedM31) -> Self::Output {
        VectorizedM31 {
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

impl Sub for VectorizedM31 {
    type Output = VectorizedM31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn sub(self, rhs: VectorizedM31) -> Self::Output {
        self - &rhs
    }
}

impl SubAssign<&VectorizedM31> for VectorizedM31 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: &VectorizedM31) {
        *self = *self - rhs;
    }
}

impl SubAssign for VectorizedM31 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}

impl From<u32> for VectorizedM31 {
    #[inline(always)]
    fn from(x: u32) -> Self {
        VectorizedM31 {
            v: [PackedM31::from(x); Self::VECTORIZE_SIZE],
        }
    }
}
