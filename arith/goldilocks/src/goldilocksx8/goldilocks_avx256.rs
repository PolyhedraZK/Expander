use std::{
    arch::x86_64::*,
    hash::Hash,
    io::{Read, Write},
    iter::{Product, Sum},
    mem::transmute,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::{field_common, FFTField, Field, SimdField};
use ark_std::Zero;
use ethnum::U256;
use rand::Rng;
use rand::RngCore;
use serdes::{ExpSerde, SerdeResult};

use crate::{Goldilocks, EPSILON, GOLDILOCKS_MOD};

#[allow(clippy::useless_transmute)]
const LO_32_BITS_MASK: __mmask8 = unsafe { transmute(0b0101u8) };

/// Number of Goldilocks elements in each __m256i element
const GOLDILOCKS_PACK_SIZE: usize = 4;

/// Packed field order
const PACKED_GOLDILOCKS_MOD: __m256i = unsafe { transmute([GOLDILOCKS_MOD; GOLDILOCKS_PACK_SIZE]) };

/// Packed epsilon (i.e., 2^64 % modulus)
const PACKED_EPSILON: __m256i = unsafe { transmute([EPSILON; GOLDILOCKS_PACK_SIZE]) };

/// Packed zero
const PACKED_0: __m256i = unsafe { transmute([0u64; GOLDILOCKS_PACK_SIZE]) };

/// Packed inverse of 2
const PACKED_INV_2: __m256i = unsafe { transmute([0x7FFFFFFF80000001u64; GOLDILOCKS_PACK_SIZE]) };

#[derive(Debug, Clone, Copy)]
pub struct AVXGoldilocks {
    // using two __m256i to simulate a __m512i
    pub v0: __m256i, // lower 4 elements
    pub v1: __m256i, // upper 4 elements
}

impl Default for AVXGoldilocks {
    fn default() -> Self {
        Self::zero()
    }
}

impl PartialEq for AVXGoldilocks {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            let pcmp0 =
                _mm256_cmpeq_epi32_mask(mod_reduce_epi64(self.v0), mod_reduce_epi64(other.v0));
            let pcmp1 =
                _mm256_cmpeq_epi32_mask(mod_reduce_epi64(self.v1), mod_reduce_epi64(other.v1));
            pcmp0 == 0xFF && pcmp1 == 0xFF
        }
    }
}

impl Eq for AVXGoldilocks {}

impl Hash for AVXGoldilocks {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        unsafe {
            state.write(transmute::<__m256i, [u8; 32]>(self.v0).as_ref());
            state.write(transmute::<__m256i, [u8; 32]>(self.v1).as_ref());
        }
    }
}

impl Neg for AVXGoldilocks {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        if self.is_zero() {
            self
        } else {
            Self {
                v0: unsafe { _mm256_sub_epi64(PACKED_GOLDILOCKS_MOD, self.v0) },
                v1: unsafe { _mm256_sub_epi64(PACKED_GOLDILOCKS_MOD, self.v1) },
            }
        }
    }
}

impl From<u32> for AVXGoldilocks {
    #[inline(always)]
    fn from(x: u32) -> Self {
        Self {
            v0: unsafe { _mm256_set1_epi64x(x as i64) },
            v1: unsafe { _mm256_set1_epi64x(x as i64) },
        }
    }
}

impl From<u64> for AVXGoldilocks {
    #[inline(always)]
    fn from(x: u64) -> Self {
        Self {
            v0: unsafe { _mm256_set1_epi64x(x as i64) },
            v1: unsafe { _mm256_set1_epi64x(x as i64) },
        }
    }
}

impl From<Goldilocks> for AVXGoldilocks {
    #[inline(always)]
    fn from(x: Goldilocks) -> Self {
        Self {
            v0: unsafe { _mm256_set1_epi64x(x.v as u64 as i64) },
            v1: unsafe { _mm256_set1_epi64x(x.v as u64 as i64) },
        }
    }
}

impl Mul<Goldilocks> for AVXGoldilocks {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Goldilocks) -> Self::Output {
        let rhs_packed = Self::from(rhs);
        self * rhs_packed
    }
}

field_common!(AVXGoldilocks);

impl Field for AVXGoldilocks {
    const NAME: &'static str = "AVXGoldilocks";

    const SIZE: usize = GOLDILOCKS_PACK_SIZE * 2 * 8; // 8 elements total (4 per register)

    const ZERO: Self = Self {
        v0: PACKED_0,
        v1: PACKED_0,
    };

    const ONE: Self = Self {
        v0: unsafe { transmute::<[u64; 4], __m256i>([1; GOLDILOCKS_PACK_SIZE]) },
        v1: unsafe { transmute::<[u64; 4], __m256i>([1; GOLDILOCKS_PACK_SIZE]) },
    };

    const INV_2: Self = Self {
        v0: PACKED_INV_2,
        v1: PACKED_INV_2,
    };

    const FIELD_SIZE: usize = 64;

    const MODULUS: U256 = U256([GOLDILOCKS_MOD as u128, 0]);

    #[inline(always)]
    fn zero() -> Self {
        Self::ZERO
    }

    #[inline(always)]
    fn one() -> Self {
        Self::ONE
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        unsafe {
            _mm256_test_epi64_mask(self.v0, self.v0) == 0
                && _mm256_test_epi64_mask(self.v1, self.v1) == 0
        }
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl RngCore) -> Self {
        unsafe {
            let mut v0 = _mm256_setr_epi64x(
                rng.gen::<i64>(),
                rng.gen::<i64>(),
                rng.gen::<i64>(),
                rng.gen::<i64>(),
            );
            let mut v1 = _mm256_setr_epi64x(
                rng.gen::<i64>(),
                rng.gen::<i64>(),
                rng.gen::<i64>(),
                rng.gen::<i64>(),
            );
            v0 = mod_reduce_epi64(v0);
            v1 = mod_reduce_epi64(v1);
            Self { v0, v1 }
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl RngCore) -> Self {
        unsafe {
            let v0 = _mm256_setr_epi64x(
                rng.gen::<bool>() as i64,
                rng.gen::<bool>() as i64,
                rng.gen::<bool>() as i64,
                rng.gen::<bool>() as i64,
            );
            let v1 = _mm256_setr_epi64x(
                rng.gen::<bool>() as i64,
                rng.gen::<bool>() as i64,
                rng.gen::<bool>() as i64,
                rng.gen::<bool>() as i64,
            );
            Self { v0, v1 }
        }
    }

    #[inline(always)]
    fn square(&self) -> Self {
        let (hi0, lo0) = square64(self.v0);
        let (hi1, lo1) = square64(self.v1);
        AVXGoldilocks {
            v0: reduce128((hi0, lo0)),
            v1: reduce128((hi1, lo1)),
        }
    }

    #[inline(always)]
    fn exp(&self, exponent: u128) -> Self {
        let mut e = exponent;
        let mut res = Self::one();
        let mut t = *self;
        while !e.is_zero() {
            let b = e & 1;
            if b == 1 {
                res *= t;
            }
            t = t * t;
            e >>= 1;
        }
        res
    }

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        // slow, should not be used in production
        let mut goldilocks_vec1 =
            unsafe { transmute::<__m256i, [Goldilocks; GOLDILOCKS_PACK_SIZE]>(self.v0) };
        let is_non_zero = goldilocks_vec1.iter().all(|x| !x.is_zero());
        if !is_non_zero {
            return None;
        }
        let mut goldilocks_vec2 =
            unsafe { transmute::<__m256i, [Goldilocks; GOLDILOCKS_PACK_SIZE]>(self.v1) };
        let is_non_zero = goldilocks_vec2.iter().all(|x| !x.is_zero());
        if !is_non_zero {
            return None;
        }

        goldilocks_vec1
            .iter_mut()
            .for_each(|x| *x = x.inv().unwrap()); // safe unwrap
        goldilocks_vec2
            .iter_mut()
            .for_each(|x| *x = x.inv().unwrap()); // safe unwrap

        let v0 =
            unsafe { transmute::<[Goldilocks; GOLDILOCKS_PACK_SIZE], __m256i>(goldilocks_vec1) };
        let v1 =
            unsafe { transmute::<[Goldilocks; GOLDILOCKS_PACK_SIZE], __m256i>(goldilocks_vec2) };
        Some(Self { v0, v1 })
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        unimplemented!("self is a vector, cannot convert to u32")
    }

    #[inline(always)]
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        let m = Goldilocks::from_uniform_bytes(bytes);
        Self {
            v0: unsafe { _mm256_set1_epi64x(m.v as i64) },
            v1: unsafe { _mm256_set1_epi64x(m.v as i64) },
        }
    }

    #[inline(always)]
    fn mul_by_5(&self) -> Self {
        *self
            * Self {
                v0: unsafe { _mm256_set1_epi64x(5) },
                v1: unsafe { _mm256_set1_epi64x(5) },
            }
    }

    #[inline(always)]
    fn mul_by_6(&self) -> Self {
        *self
            * Self {
                v0: unsafe { _mm256_set1_epi64x(6) },
                v1: unsafe { _mm256_set1_epi64x(6) },
            }
    }
}

impl FFTField for AVXGoldilocks {
    const TWO_ADICITY: usize = 32;

    fn root_of_unity() -> Self {
        Self {
            v0: unsafe { _mm256_set1_epi64x(0x185629dcda58878c) },
            v1: unsafe { _mm256_set1_epi64x(0x185629dcda58878c) },
        }
    }
}

impl ExpSerde for AVXGoldilocks {
    const SERIALIZED_SIZE: usize = GOLDILOCKS_PACK_SIZE * 2 * 8; // 64 bytes total

    #[inline(always)]
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        let data0 = unsafe { transmute::<__m256i, [u8; 32]>(self.v0) };
        let data1 = unsafe { transmute::<__m256i, [u8; 32]>(self.v1) };
        writer.write_all(&data0)?;
        writer.write_all(&data1)?;
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let mut data0 = [0; 32];
        let mut data1 = [0; 32];
        reader.read_exact(&mut data0)?;
        reader.read_exact(&mut data1)?;
        unsafe {
            let v0 = transmute::<[u8; 32], __m256i>(data0);
            let v1 = transmute::<[u8; 32], __m256i>(data1);
            Ok(Self { v0, v1 })
        }
    }
}

impl SimdField for AVXGoldilocks {
    type Scalar = Goldilocks;

    #[inline]
    fn scale(&self, challenge: &Self::Scalar) -> Self {
        *self * *challenge
    }

    const PACK_SIZE: usize = GOLDILOCKS_PACK_SIZE;

    #[inline(always)]
    fn pack(base_vec: &[Self::Scalar]) -> Self {
        assert_eq!(base_vec.len(), Self::PACK_SIZE);
        let ret: [Self::Scalar; Self::PACK_SIZE] = base_vec.try_into().unwrap();
        let v0 = unsafe { transmute::<[Self::Scalar; Self::PACK_SIZE], __m256i>(ret) };
        let v1 = unsafe { transmute::<[Self::Scalar; Self::PACK_SIZE], __m256i>(ret) };
        Self { v0, v1 }
    }

    #[inline(always)]
    fn unpack(&self) -> Vec<Self::Scalar> {
        let ret = unsafe {
            transmute::<[__m256i; 2], [Self::Scalar; Self::PACK_SIZE]>([self.v0, self.v1])
        };
        ret.to_vec()
    }
}

#[inline]
unsafe fn mod_reduce_epi64(x: __m256i) -> __m256i {
    let mask = _mm256_cmpgt_epu64_mask(x, PACKED_GOLDILOCKS_MOD);
    _mm256_mask_sub_epi64(x, mask, x, PACKED_GOLDILOCKS_MOD)
}

#[inline(always)]
fn add_internal(a: &AVXGoldilocks, b: &AVXGoldilocks) -> AVXGoldilocks {
    AVXGoldilocks {
        v0: add_no_double_overflow_64_64(a.v0, canonicalize(b.v0)),
        v1: add_no_double_overflow_64_64(a.v1, canonicalize(b.v1)),
    }
}

#[inline(always)]
fn sub_internal(a: &AVXGoldilocks, b: &AVXGoldilocks) -> AVXGoldilocks {
    AVXGoldilocks {
        v0: sub_no_double_overflow_64_64(a.v0, canonicalize(b.v0)),
        v1: sub_no_double_overflow_64_64(a.v1, canonicalize(b.v1)),
    }
}

#[inline]
fn mul_internal(x: &AVXGoldilocks, y: &AVXGoldilocks) -> AVXGoldilocks {
    let (hi0, lo0) = mul64_64(x.v0, y.v0);
    let (hi1, lo1) = mul64_64(x.v1, y.v1);
    AVXGoldilocks {
        v0: reduce128((hi0, lo0)),
        v1: reduce128((hi1, lo1)),
    }
}

#[inline]
fn canonicalize(x: __m256i) -> __m256i {
    unsafe {
        let mask = _mm256_cmpge_epu64_mask(x, PACKED_GOLDILOCKS_MOD);
        _mm256_mask_sub_epi64(x, mask, x, PACKED_GOLDILOCKS_MOD)
    }
}

#[inline]
fn mul64_64(x: __m256i, y: __m256i) -> (__m256i, __m256i) {
    unsafe {
        let x_hi = _mm256_castps_si256(_mm256_movehdup_ps(_mm256_castsi256_ps(x)));
        let y_hi = _mm256_castps_si256(_mm256_movehdup_ps(_mm256_castsi256_ps(y)));

        let mul_ll = _mm256_mul_epu32(x, y);
        let mul_lh = _mm256_mul_epu32(x, y_hi);
        let mul_hl = _mm256_mul_epu32(x_hi, y);
        let mul_hh = _mm256_mul_epu32(x_hi, y_hi);

        let mul_ll_hi = _mm256_srli_epi64::<32>(mul_ll);
        let t0 = _mm256_add_epi64(mul_hl, mul_ll_hi);
        let t0_lo = _mm256_and_si256(t0, PACKED_EPSILON);
        let t0_hi = _mm256_srli_epi64::<32>(t0);
        let t1 = _mm256_add_epi64(mul_lh, t0_lo);
        let t2 = _mm256_add_epi64(mul_hh, t0_hi);
        let t1_hi = _mm256_srli_epi64::<32>(t1);
        let res_hi = _mm256_add_epi64(t2, t1_hi);

        let t1_lo = _mm256_castps_si256(_mm256_moveldup_ps(_mm256_castsi256_ps(t1)));
        let res_lo = _mm256_mask_blend_epi32(LO_32_BITS_MASK, t1_lo, mul_ll);

        (res_hi, res_lo)
    }
}

#[inline]
fn reduce128(x: (__m256i, __m256i)) -> __m256i {
    unsafe {
        let (hi0, lo0) = x;
        let hi_hi0 = _mm256_srli_epi64::<32>(hi0);
        let lo1 = sub_no_double_overflow_64_64(lo0, hi_hi0);
        let t1 = _mm256_mul_epu32(hi0, PACKED_EPSILON);
        add_no_double_overflow_64_64(lo1, t1)
    }
}

#[inline]
fn sub_no_double_overflow_64_64(x: __m256i, y: __m256i) -> __m256i {
    unsafe {
        let mask = _mm256_cmplt_epu64_mask(x, y);
        let res_wrapped = _mm256_sub_epi64(x, y);
        _mm256_mask_add_epi64(res_wrapped, mask, res_wrapped, PACKED_GOLDILOCKS_MOD)
    }
}

#[inline]
fn add_no_double_overflow_64_64(x: __m256i, y: __m256i) -> __m256i {
    unsafe {
        let res_wrapped = _mm256_add_epi64(x, y);
        let mask = _mm256_cmplt_epu64_mask(res_wrapped, y);
        _mm256_mask_sub_epi64(res_wrapped, mask, res_wrapped, PACKED_GOLDILOCKS_MOD)
    }
}

#[inline]
fn square64(x: __m256i) -> (__m256i, __m256i) {
    unsafe {
        let x_hi = _mm256_castps_si256(_mm256_movehdup_ps(_mm256_castsi256_ps(x)));

        let mul_ll = _mm256_mul_epu32(x, x);
        let mul_lh = _mm256_mul_epu32(x, x_hi);
        let mul_hh = _mm256_mul_epu32(x_hi, x_hi);

        let mul_ll_hi = _mm256_srli_epi64::<33>(mul_ll);
        let t0 = _mm256_add_epi64(mul_lh, mul_ll_hi);
        let t0_hi = _mm256_srli_epi64::<31>(t0);
        let res_hi = _mm256_add_epi64(mul_hh, t0_hi);

        let mul_lh_lo = _mm256_slli_epi64::<33>(mul_lh);
        let res_lo = _mm256_add_epi64(mul_ll, mul_lh_lo);

        (res_hi, res_lo)
    }
}
