use std::{
    arch::x86_64::*,
    fmt::Debug,
    mem::size_of,
    ops::{Add, AddAssign, Mul},
};

use crate::{Field, M31, M31_MOD};
use lazy_static::lazy_static;

pub type PackedDataType = __m256i;
pub const M31_PACK_SIZE: usize = 8;
pub const M31_VECTORIZE_SIZE: usize = 1;
lazy_static! {
    pub static ref PACKED_MOD: __m256i = unsafe { _mm256_set1_epi32(M31_MOD) };
    pub static ref PACKED_0: __m256i = unsafe { _mm256_setzero_si256() };
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
use rand::Rng;

#[derive(Clone, Copy)]
pub struct PackedM31 {
    v: PackedDataType,
}

impl PackedM31 {
    pub const SIZE: usize = size_of::<PackedDataType>();
    pub fn pack_full(x: M31) -> PackedM31 {
        PackedM31 {
            v: unsafe { _mm256_set1_epi32(x.v as i32) },
        }
    }
}

impl Field for PackedM31 {
    fn zero() -> Self {
        PackedM31 {
            v: unsafe { _mm256_set1_epi32(0) },
        }
    }

    fn one() -> Self {
        todo!();
    }

    fn random() -> Self {
        unsafe {
            let mut rng = rand::thread_rng();
            let mut v = _mm256_setr_epi32(
                rng.gen::<i32>(),
                rng.gen::<i32>(),
                rng.gen::<i32>(),
                rng.gen::<i32>(),
                rng.gen::<i32>(),
                rng.gen::<i32>(),
                rng.gen::<i32>(),
                rng.gen::<i32>(),
            );
            v = mod_reduce_epi32(v);
            PackedM31 {
                v: _mm256_mask_sub_epi32(
                    v,
                    _mm256_cmpge_epu32_mask(v, *PACKED_MOD),
                    v,
                    *PACKED_MOD,
                ),
            }
        }
    }

    fn random_bool() -> Self {
        let mut rng = rand::thread_rng();
        PackedM31 {
            v: unsafe {
                _mm256_setr_epi32(
                    rng.gen::<bool>() as i32,
                    rng.gen::<bool>() as i32,
                    rng.gen::<bool>() as i32,
                    rng.gen::<bool>() as i32,
                    rng.gen::<bool>() as i32,
                    rng.gen::<bool>() as i32,
                    rng.gen::<bool>() as i32,
                    rng.gen::<bool>() as i32,
                )
            },
        }
    }

    fn inv(&self) -> Self {
        todo!();
    }
}

impl Debug for PackedM31 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut data = [0; M31_PACK_SIZE];
        unsafe {
            _mm256_storeu_si256(data.as_mut_ptr() as *mut PackedDataType, self.v);
        }
        write!(f, "mm256i<{:?}>", data)
    }
}

impl Default for PackedM31 {
    fn default() -> Self {
        PackedM31::zero()
    }
}

impl PartialEq for PackedM31 {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            let pcmp = _mm256_cmpeq_epi32(self.v, other.v);
            _mm256_movemask_epi8(pcmp) == 0xffffffffu32 as i32
        }
    }
}

impl Mul<&PackedM31> for PackedM31 {
    type Output = PackedM31;
    fn mul(self, rhs: &PackedM31) -> Self::Output {
        unsafe {
            let x_shifted = _mm256_srli_epi64::<32>(self.v);
            let rhs_shifted = _mm256_srli_epi64::<32>(rhs.v);
            let mut xa_even = _mm256_mul_epi32(self.v, rhs.v);
            let mut xa_odd = _mm256_mul_epi32(x_shifted, rhs_shifted);
            xa_even = mod_reduce(xa_even);
            xa_odd = mod_reduce(xa_odd);
            PackedM31 {
                v: mod_reduce_epi32(_mm256_or_si256(xa_even, _mm256_slli_epi64::<32>(xa_odd))),
            }
        }
    }
}

impl Mul for PackedM31 {
    type Output = PackedM31;
    fn mul(self, rhs: PackedM31) -> Self::Output {
        self * &rhs
    }
}

impl Add<&PackedM31> for PackedM31 {
    type Output = PackedM31;
    fn add(self, rhs: &PackedM31) -> Self::Output {
        unsafe {
            let result = _mm256_add_epi32(self.v, rhs.v);
            PackedM31 {
                v: _mm256_mask_sub_epi32(
                    result,
                    _mm256_cmpge_epu32_mask(result, *PACKED_MOD),
                    result,
                    *PACKED_MOD,
                ),
            }
        }
    }
}

impl AddAssign<&PackedM31> for PackedM31 {
    fn add_assign(&mut self, rhs: &PackedM31) {
        *self = *self + rhs;
    }
}

impl AddAssign for PackedM31 {
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl From<usize> for PackedM31 {
    fn from(x: usize) -> Self {
        PackedM31::pack_full(M31::from(x))
    }
}
