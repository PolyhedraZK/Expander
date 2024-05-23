use std::{
    arch::x86_64::*,
    fmt::Debug,
    iter::{Product, Sum},
    mem::{size_of, transmute},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use crate::{Field, M31, M31_MOD};

type PackedDataType = __m256i;
pub const M31_PACK_SIZE: usize = 8;
pub const M31_VECTORIZE_SIZE: usize = 1;

const PACKED_MOD: __m256i = unsafe { transmute([M31_MOD; 8]) };
const PACKED_0: __m256i = unsafe { transmute([0; 8]) };
const PACKED_MOD_EPI64: __m256i = unsafe { transmute([M31_MOD as u64; 4]) };
const _PACKED_MOD_SQUARE: __m256 = unsafe { transmute([(M31_MOD as u64 * M31_MOD as u64); 4]) };
const _PACKED_MOD_512: __m512i = unsafe { transmute([M31_MOD as i64; 8]) };
pub(crate) const PACKED_INV_2: __m256i = unsafe { transmute([1 << 30; 8]) };

#[inline(always)]
unsafe fn mod_reduce_epi64(x: __m256i) -> __m256i {
    _mm256_add_epi64(
        _mm256_and_si256(x, PACKED_MOD_EPI64),
        _mm256_srli_epi64(x, 31),
    )
}

#[inline(always)]
unsafe fn mod_reduce_epi32(x: __m256i) -> __m256i {
    _mm256_add_epi32(_mm256_and_si256(x, PACKED_MOD), _mm256_srli_epi32(x, 31))
}

use mod_reduce_epi64 as mod_reduce;
use rand::Rng;

#[derive(Clone, Copy)]
pub struct PackedM31 {
    pub v: PackedDataType,
}

impl PackedM31 {
    pub const SIZE: usize = size_of::<PackedDataType>();
    #[inline(always)]
    pub fn pack_full(x: M31) -> PackedM31 {
        PackedM31 {
            v: unsafe { _mm256_set1_epi32(x.v as i32) },
        }
    }
}

impl Field for PackedM31 {
    const NAME: &'static str = "AVX Packed Mersenne 31";

    const SIZE: usize = size_of::<PackedDataType>();

    type BaseField = M31;

    #[inline(always)]
    fn zero() -> Self {
        PackedM31 {
            v: unsafe { _mm256_set1_epi32(0) },
        }
    }

    #[inline(always)]
    fn one() -> Self {
        PackedM31 {
            v: unsafe { _mm256_set1_epi32(1) },
        }
    }

    #[inline(always)]
    // this function is for internal testing only. it is not
    // a source for uniformly random field elements and
    // should not be used in production.
    fn random() -> Self {
        // Caution: this may not produce uniformly random elements
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
                v: _mm256_mask_sub_epi32(v, _mm256_cmpge_epu32_mask(v, PACKED_MOD), v, PACKED_MOD),
            }
        }
    }

    #[inline(always)]
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

    #[inline(always)]
    fn inv(&self) -> Self {
        todo!();
    }

    #[inline(always)]
    fn mul_by_base(&self, rhs: &Self::BaseField) -> Self {
        *self * rhs
    }

    fn as_u32_unchecked(&self)-> u32{
        unimplemented!("self is a vector, cannot convert to u32")
    }
}

impl Debug for PackedM31 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut data = [0; M31_PACK_SIZE];
        unsafe {
            _mm256_storeu_si256(data.as_mut_ptr() as *mut PackedDataType, self.v);
        }
        // if all data is the same, print only one
        if data.iter().all(|&x| x == data[0]) {
            write!(
                f,
                "mm256i<8 x {}>",
                if M31_MOD - data[0] > 1024 {
                    format!("{}", data[0])
                } else {
                    format!("-{}", M31_MOD - data[0])
                }
            )
        } else {
            write!(f, "mm256i<{:?}>", data)
        }
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
    #[inline(always)]
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
    #[inline(always)]
    fn mul(self, rhs: PackedM31) -> Self::Output {
        self * &rhs
    }
}

impl Mul<&M31> for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    fn mul(self, rhs: &M31) -> Self::Output {
        unsafe {
            let rhs_p = _mm256_set1_epi32(rhs.v as i32);
            self * &PackedM31 { v: rhs_p }
        }
    }
}

impl Mul<M31> for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    fn mul(self, rhs: M31) -> Self::Output {
        self * &rhs
    }
}

impl MulAssign<&PackedM31> for PackedM31 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: &PackedM31) {
        *self = *self * rhs;
    }
}

impl MulAssign for PackedM31 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        *self *= &rhs;
    }
}

impl<T: ::core::borrow::Borrow<PackedM31>> Product<T> for PackedM31 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Add<&PackedM31> for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    fn add(self, rhs: &PackedM31) -> Self::Output {
        unsafe {
            let result = _mm256_add_epi32(self.v, rhs.v);
            PackedM31 {
                v: _mm256_mask_sub_epi32(
                    result,
                    _mm256_cmpge_epu32_mask(result, PACKED_MOD),
                    result,
                    PACKED_MOD,
                ),
            }
        }
    }
}

impl Add for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    fn add(self, rhs: PackedM31) -> Self::Output {
        self + &rhs
    }
}

impl AddAssign<&PackedM31> for PackedM31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &PackedM31) {
        *self = *self + rhs;
    }
}

impl AddAssign for PackedM31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl<T: ::core::borrow::Borrow<PackedM31>> Sum<T> for PackedM31 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl From<u32> for PackedM31 {
    #[inline(always)]
    fn from(x: u32) -> Self {
        PackedM31::pack_full(M31::from(x))
    }
}

impl Neg for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        PackedM31 { v: PACKED_0 } - self
    }
}

impl Sub<&PackedM31> for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    fn sub(self, rhs: &PackedM31) -> Self::Output {
        PackedM31 {
            v: unsafe {
                let t = _mm256_sub_epi32(self.v, rhs.v);
                _mm256_mask_add_epi32(t, _mm256_cmpge_epu32_mask(t, PACKED_MOD), t, PACKED_MOD)
            },
        }
    }
}

impl Sub for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    fn sub(self, rhs: PackedM31) -> Self::Output {
        self - &rhs
    }
}

impl SubAssign<&PackedM31> for PackedM31 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: &PackedM31) {
        *self = *self - rhs;
    }
}

impl SubAssign for PackedM31 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}
