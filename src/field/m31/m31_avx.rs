use std::{
    arch::x86_64::*,
    ops::{AddAssign, Mul},
};

use crate::{Field, M31_MOD};
use lazy_static::lazy_static;

pub const M31_PACK_SIZE: usize = 8;
pub const M31_VECTORIZE_SIZE: usize = 1;
lazy_static! {
    pub static ref PACKED_MOD: __m256i = unsafe { _mm256_set1_epi32(M31_MOD) };
    pub static ref PACKED_0: __m256i = unsafe { _mm256_set1_epi32(0) };
    pub static ref PACKED_MOD_EPI64: __m256i = unsafe { _mm256_set1_epi64x(M31_MOD as i64) };
    pub static ref PACKED_MOD_SQUARE: __m256i =
        unsafe { _mm256_set1_epi64x((M31_MOD as i64) * (M31_MOD as i64)) };
    pub static ref PACKED_MOD_512: __m512i = unsafe { _mm512_set1_epi64(M31_MOD as i64) };
}

#[inline(always)]
unsafe fn mod_reduce_epi64(x: __m256i) -> __m256i {
    _mm256_add_epi64(
        _mm256_and_si256(x, *PACKED_MOD_EPI64),
        _mm256_srli_epi64(x, 31),
    )
}

#[inline(always)]
unsafe fn mod_reduce_epi32(x: __m256i) -> __m256i {
    _mm256_add_epi32(_mm256_and_si256(x, *PACKED_MOD), _mm256_srli_epi32(x, 31))
}

use mod_reduce_epi64 as mod_reduce;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct M31 {}

impl Field for M31 {
    fn zero() -> Self {
        M31 {} // TODO
    }

    fn one() -> Self {
        M31 {} // TODO
    }

    fn random() -> Self {
        M31 {} // TODO
    }

    fn random_bool() -> Self {
        todo!()
    }

    fn inv(&self) -> Self {
        M31 {} // TODO
    }
}

impl Mul for M31 {
    type Output = M31;
    fn mul(self, rhs: M31) -> Self::Output {
        M31 {} // TODO
    }
}

impl AddAssign for M31 {
    fn add_assign(&mut self, rhs: Self) {
        // TODO
    }
}

impl From<usize> for M31 {
    fn from(x: usize) -> Self {
        M31 {} // TODO
    }
}
