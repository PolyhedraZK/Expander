use std::{
    arch::x86_64::*,
    fmt::Debug,
    io::{Read, Write},
    iter::{Product, Sum},
    mem::transmute,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use rand::{Rng, RngCore};

use crate::{FiatShamirConfig, Field, FieldSerde, M31, M31_MOD};

const M31_PACK_SIZE: usize = 8;
const PACKED_MOD: __m256i = unsafe { transmute([M31_MOD; M31_PACK_SIZE]) };
const PACKED_0: __m256i = unsafe { transmute([0; M31_PACK_SIZE]) };
const PACKED_MOD_EPI64: __m256i = unsafe { transmute([M31_MOD as u64; 4]) };
const PACKED_INV_2: __m256i = unsafe { transmute([1 << 30; M31_PACK_SIZE]) };

pub(crate) const FIVE: AVXM31 = AVXM31 {
    v: unsafe { transmute::<[i32; 8], std::arch::x86_64::__m256i>([5; 8]) },
};

pub(crate) const TEN: AVXM31 = AVXM31 {
    v: unsafe { transmute::<[i32; 8], std::arch::x86_64::__m256i>([10; 8]) },
};

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

#[derive(Clone, Copy)]
pub struct AVXM31 {
    pub v: __m256i,
}

impl AVXM31 {
    #[inline(always)]
    pub(crate) fn pack_full(x: M31) -> AVXM31 {
        AVXM31 {
            v: unsafe { _mm256_set1_epi32(x.v as i32) },
        }
    }
}

impl FieldSerde for AVXM31 {
    #[inline(always)]
    /// serialize self into bytes
    fn serialize_into<W: Write>(&self, mut writer: W) {
        let data = unsafe { transmute::<__m256i, [u8; 32]>(self.v) };
        writer.write_all(&data).unwrap();
    }

    #[inline(always)]
    fn serialized_size() -> usize {
        32
    }

    /// deserialize bytes into field
    #[inline(always)]
    fn deserialize_from<R: Read>(mut reader: R) -> Self {
        let mut data = [0; 32];
        reader.read_exact(&mut data).unwrap();
        unsafe {
            AVXM31 {
                v: transmute::<[u8; 32], __m256i>(data),
            }
        }
    }

    #[inline(always)]
    fn deserialize_from_ecc_format<R: Read>(mut reader: R) -> Self {
        let mut buf = [0u8; 32];
        reader.read_exact(&mut buf).unwrap(); // todo: error propagation
        assert!(
            buf.iter().skip(4).all(|&x| x == 0),
            "non-zero byte found in witness byte"
        );
        Self::pack_full(u32::from_le_bytes(buf[..4].try_into().unwrap()).into())
    }
}

impl Field for AVXM31 {
    const NAME: &'static str = "AVX Packed Mersenne 31";

    // size in bytes
    const SIZE: usize = 32;

    const INV_2: Self = Self { v: PACKED_INV_2 };

    #[inline(always)]
    fn zero() -> Self {
        AVXM31 {
            v: unsafe { _mm256_set1_epi32(0) },
        }
    }

    #[inline(always)]
    fn one() -> Self {
        AVXM31 {
            v: unsafe { _mm256_set1_epi32(1) },
        }
    }

    #[inline(always)]
    // this function is for internal testing only. it is not
    // a source for uniformly random field elements and
    // should not be used in production.
    fn random_unsafe(mut rng: impl RngCore) -> Self {
        // Caution: this may not produce uniformly random elements
        unsafe {
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
            v = mod_reduce_epi32(v);
            AVXM31 { v }
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl RngCore) -> Self {
        // TODO: optimize this code
        AVXM31 {
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

    fn exp(&self, _exponent: &Self) -> Self {
        unimplemented!("exp not implemented for AVXM31")
    }

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        // slow, should not be used in production
        let mut m31_vec = unsafe { transmute::<__m256i, [M31; 8]>(self.v) };
        let is_non_zero = m31_vec.iter().all(|x| !x.is_zero());
        if !is_non_zero {
            return None;
        }

        m31_vec.iter_mut().for_each(|x| *x = x.inv().unwrap()); // safe unwrap
        Some(Self {
            v: unsafe { transmute::<[M31; 8], __m256i>(m31_vec) },
        })
    }

    fn as_u32_unchecked(&self) -> u32 {
        unimplemented!("self is a vector, cannot convert to u32")
    }

    #[inline]
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        let m = M31::from_uniform_bytes(bytes);
        Self {
            v: unsafe { _mm256_set1_epi32(m.v as i32) },
        }
    }
}

impl FiatShamirConfig for AVXM31 {
    type ChallengeField = M31;

    #[inline]
    fn scale(&self, challenge: &Self::ChallengeField) -> Self {
        *self * *challenge
    }
}

impl From<M31> for AVXM31 {
    #[inline(always)]
    fn from(x: M31) -> Self {
        AVXM31::pack_full(x)
    }
}

impl Debug for AVXM31 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut data = [0; M31_PACK_SIZE];
        unsafe {
            _mm256_storeu_si256(data.as_mut_ptr() as *mut __m256i, self.v);
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

impl Default for AVXM31 {
    fn default() -> Self {
        AVXM31::zero()
    }
}

impl PartialEq for AVXM31 {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            let pcmp = _mm256_cmpeq_epi32(self.v, other.v);
            _mm256_movemask_epi8(pcmp) == 0xffffffffu32 as i32
        }
    }
}

impl Mul<&AVXM31> for AVXM31 {
    type Output = AVXM31;
    #[inline(always)]
    fn mul(self, rhs: &AVXM31) -> Self::Output {
        unsafe {
            let x_shifted = _mm256_srli_epi64::<32>(self.v);
            let rhs_shifted = _mm256_srli_epi64::<32>(rhs.v);
            let mut xa_even = _mm256_mul_epi32(self.v, rhs.v);
            let mut xa_odd = _mm256_mul_epi32(x_shifted, rhs_shifted);
            xa_even = mod_reduce_epi64(xa_even);
            xa_odd = mod_reduce_epi64(xa_odd);
            AVXM31 {
                v: mod_reduce_epi32(_mm256_or_si256(xa_even, _mm256_slli_epi64::<32>(xa_odd))),
            }
        }
    }
}

impl Mul for AVXM31 {
    type Output = AVXM31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn mul(self, rhs: AVXM31) -> Self::Output {
        self * &rhs
    }
}

impl Mul<&M31> for AVXM31 {
    type Output = AVXM31;
    #[inline(always)]
    fn mul(self, rhs: &M31) -> Self::Output {
        unsafe {
            let rhs_p = _mm256_set1_epi32(rhs.v as i32);
            self * AVXM31 { v: rhs_p }
        }
    }
}

impl Mul<M31> for AVXM31 {
    type Output = AVXM31;
    #[inline(always)]
    fn mul(self, rhs: M31) -> Self::Output {
        self * &rhs
    }
}

impl MulAssign<&AVXM31> for AVXM31 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: &AVXM31) {
        *self = *self * rhs;
    }
}

impl MulAssign for AVXM31 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        *self *= &rhs;
    }
}

impl<T: ::core::borrow::Borrow<AVXM31>> Product<T> for AVXM31 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Add<&AVXM31> for AVXM31 {
    type Output = AVXM31;
    #[inline(always)]
    fn add(self, rhs: &AVXM31) -> Self::Output {
        unsafe {
            let mut result = _mm256_add_epi32(self.v, rhs.v);
            let subx = _mm256_sub_epi32(result, PACKED_MOD);
            result = _mm256_min_epu32(result, subx);

            AVXM31 { v: result }
        }
    }
}

impl Add for AVXM31 {
    type Output = AVXM31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn add(self, rhs: AVXM31) -> Self::Output {
        self + &rhs
    }
}

impl AddAssign<&AVXM31> for AVXM31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &AVXM31) {
        *self = *self + rhs;
    }
}

impl AddAssign for AVXM31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl<T: ::core::borrow::Borrow<AVXM31>> Sum<T> for AVXM31 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl From<u32> for AVXM31 {
    #[inline(always)]
    fn from(x: u32) -> Self {
        AVXM31::pack_full(M31::from(x))
    }
}

impl Neg for AVXM31 {
    type Output = AVXM31;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        unsafe {
            let subx = _mm256_sub_epi32(PACKED_MOD, self.v);
            let zero_cmp = _mm256_cmpeq_epi32(self.v, PACKED_0);
            AVXM31 {
                v: _mm256_andnot_si256(zero_cmp, subx),
            }
        }
    }
}

impl Sub<&AVXM31> for AVXM31 {
    type Output = AVXM31;
    #[inline(always)]
    fn sub(self, rhs: &AVXM31) -> Self::Output {
        AVXM31 {
            v: unsafe {
                let t = _mm256_sub_epi32(self.v, rhs.v);
                let subx = _mm256_add_epi32(t, PACKED_MOD);
                _mm256_min_epu32(t, subx)
            },
        }
    }
}

impl Sub for AVXM31 {
    type Output = AVXM31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn sub(self, rhs: AVXM31) -> Self::Output {
        self - &rhs
    }
}

impl SubAssign<&AVXM31> for AVXM31 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: &AVXM31) {
        *self = *self - rhs;
    }
}

impl SubAssign for AVXM31 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}
