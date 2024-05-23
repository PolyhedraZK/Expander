#[cfg(target_arch = "x86_64")]
pub mod m31_avx;
#[cfg(target_arch = "x86_64")]
pub use m31_avx::{PackedM31, M31_PACK_SIZE, M31_VECTORIZE_SIZE};

#[cfg(target_arch = "aarch64")]
pub mod m31_neon;
#[cfg(target_arch = "aarch64")]
pub use m31_neon::{PackedM31, M31_PACK_SIZE, M31_VECTORIZE_SIZE};
use rand::RngCore;

use crate::{Field, FieldSerde};
use std::{
    iter::{Product, Sum},
    mem::size_of,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

pub const M31_MOD: i32 = 2147483647;

#[inline]
fn mod_reduce_i32(x: i32) -> i32 {
    (x & M31_MOD) + (x >> 31)
}

#[inline]
fn mod_reduce_i64(x: i64) -> i64 {
    (x & M31_MOD as i64) + (x >> 31)
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct M31 {
    pub v: u32,
}

impl FieldSerde for M31 {
    #[inline(always)]
    fn serialize_into(&self, buffer: &mut [u8]) {
        buffer[..M31::SIZE].copy_from_slice(unsafe {
            std::slice::from_raw_parts(&self.v as *const u32 as *const u8, M31::SIZE)
        });
    }
    #[inline(always)]
    fn deserialize_from(buffer: &[u8]) -> Self {
        let ptr = buffer.as_ptr() as *const u32;

        let mut v = unsafe { ptr.read_unaligned() } as i32;
        v = mod_reduce_i32(v);
        M31 { v: v as u32 }
    }
}

impl Field for M31 {
    const NAME: &'static str = "Mersenne 31";

    const SIZE: usize = size_of::<u32>();

    const INV_2: M31 = M31 { v: 1 << 30 };

    type BaseField = M31;

    #[inline(always)]
    fn zero() -> Self {
        M31 { v: 0 }
    }

    #[inline(always)]
    fn one() -> Self {
        M31 { v: 1 }
    }

    fn random_unsafe(mut rng: impl RngCore) -> Self {
        rng.next_u32().into()
    }

    fn random_bool_unsafe(mut rng: impl RngCore) -> Self {
        (rng.next_u32() & 1).into()
    }

    fn exp(&self) -> Self {
        todo!()
    }

    fn inv(&self) -> Option<Self> {
        todo!()
    }

    #[inline(always)]
    fn add_base_elem(&self, rhs: &Self::BaseField) -> Self {
        *self + *rhs
    }

    #[inline(always)]
    fn add_assign_base_elem(&mut self, rhs: &Self::BaseField) {
        *self += rhs
    }

    #[inline(always)]
    fn mul_base_elem(&self, rhs: &Self::BaseField) -> Self {
        *self * rhs
    }

    #[inline(always)]
    fn mul_assign_base_elem(&mut self, rhs: &Self::BaseField) {
        *self *= rhs;
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        self.v
    }
}

// ====================================
// Arithmetics for M31
// ====================================

impl Mul<&M31> for M31 {
    type Output = M31;
    #[inline(always)]
    fn mul(self, rhs: &M31) -> Self::Output {
        let mut vv = self.v as i64 * rhs.v as i64;
        vv = mod_reduce_i64(vv);

        // ZZ: this seems unnecessary since it is already reduced?
        if vv >= M31_MOD as i64 {
            vv -= M31_MOD as i64;
        }
        M31 { v: vv as u32 }
    }
}

impl Mul for M31 {
    type Output = M31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn mul(self, rhs: M31) -> Self::Output {
        self * &rhs
    }
}

impl MulAssign<&M31> for M31 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: &M31) {
        *self = *self * rhs;
    }
}

impl MulAssign for M31 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        *self *= &rhs;
    }
}

impl<T: ::core::borrow::Borrow<M31>> Product<T> for M31 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Add<&M31> for M31 {
    type Output = M31;
    #[inline(always)]
    fn add(self, rhs: &M31) -> Self::Output {
        let mut vv = self.v + rhs.v;
        if vv >= M31_MOD as u32 {
            vv -= M31_MOD as u32;
        }
        M31 { v: vv }
    }
}

impl Add for M31 {
    type Output = M31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn add(self, rhs: M31) -> Self::Output {
        self + &rhs
    }
}

impl AddAssign<&M31> for M31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &M31) {
        *self = *self + rhs;
    }
}

impl AddAssign for M31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl<T: ::core::borrow::Borrow<M31>> Sum<T> for M31 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl Neg for M31 {
    type Output = M31;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        M31 {
            v: if self.v == 0 {
                0
            } else {
                M31_MOD as u32 - self.v
            },
        }
    }
}

impl Sub<&M31> for M31 {
    type Output = M31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn sub(self, rhs: &M31) -> Self::Output {
        self + &(-*rhs)
    }
}

impl Sub for M31 {
    type Output = M31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn sub(self, rhs: M31) -> Self::Output {
        self - &rhs
    }
}

impl SubAssign<&M31> for M31 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: &M31) {
        *self = *self - rhs;
    }
}

impl SubAssign for M31 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}

impl From<u32> for M31 {
    #[inline(always)]
    fn from(x: u32) -> Self {
        M31 {
            v: if x < M31_MOD as u32 {
                x
            } else {
                x % M31_MOD as u32
            },
        }
    }
}
