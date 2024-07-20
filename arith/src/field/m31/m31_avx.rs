use std::{
    arch::x86_64::*,
    fmt::Debug,
    io::{Read, Write},
    iter::{Product, Sum},
    mem::transmute,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use crate::{Field, FieldSerde, M31, M31_MOD};

type PackedDataType = __m256i;
pub(super) const M31_PACK_SIZE: usize = 8;
pub(super) const M31_VECTORIZE_SIZE: usize = 1;

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
use rand::{Rng, RngCore};

#[derive(Clone, Copy)]
pub struct PackedM31 {
    pub v: PackedDataType,
}

impl PackedM31 {
    #[inline(always)]
    pub(crate) fn pack_full(x: M31) -> PackedM31 {
        PackedM31 {
            v: unsafe { _mm256_set1_epi32(x.v as i32) },
        }
    }
}

impl FieldSerde for PackedM31 {
    #[inline(always)]
    /// serialize self into bytes
    fn serialize_into<W: Write>(&self, mut writer: W) {
        let data = unsafe { transmute::<PackedDataType, [u8; 32]>(self.v) };
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
            PackedM31 {
                v: transmute::<[u8; 32], PackedDataType>(data),
            }
        }
    }
}

impl Field for PackedM31 {
    const NAME: &'static str = "AVX Packed Mersenne 31";

    const SIZE: usize = 32;

    const INV_2: Self = Self { v: PACKED_INV_2 };

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
            PackedM31 { v }
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl RngCore) -> Self {
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

    fn exp(&self, _exponent: &Self) -> Self {
        todo!()
    }

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        unimplemented!()
    }

    // #[inline(always)]
    // fn add_base_elem(&self, _rhs: &Self::BaseField) -> Self {
    //     unimplemented!()
    // }

    #[inline(always)]
    fn add_assign_base_elem(&mut self, _rhs: &Self::BaseField) {
        unimplemented!()
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
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        let v = unsafe { transmute::<[u8; 32], __m256i>(*bytes) };
        Self { v }
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
    #[allow(clippy::op_ref)]
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
            self * PackedM31 { v: rhs_p }
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
            let mut result = _mm256_add_epi32(self.v, rhs.v);
            let subx = _mm256_sub_epi32(result, PACKED_MOD);
            result = _mm256_min_epu32(result, subx);

            PackedM31 { v: result }
        }
    }
}

impl Add for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
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
        unsafe {
            let subx = _mm256_sub_epi32(PACKED_MOD, self.v);
            let zero_cmp = _mm256_cmpeq_epi32(self.v, PACKED_0);
            PackedM31 {
                v: _mm256_andnot_si256(zero_cmp, subx),
            }
        }
    }
}

impl Sub<&PackedM31> for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    fn sub(self, rhs: &PackedM31) -> Self::Output {
        PackedM31 {
            v: unsafe {
                let t = _mm256_sub_epi32(self.v, rhs.v);
                let subx = _mm256_add_epi32(t, PACKED_MOD);
                _mm256_min_epu32(t, subx)
            },
        }
    }
}

impl Sub for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
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
