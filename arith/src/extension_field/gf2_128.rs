use std::{
    arch::x86_64::*,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use crate::{Field, FieldSerde, GF2};

use super::BinomialExtensionField;

#[derive(Debug, Clone, Copy)]
pub struct GF2_128 {
    pub v: __m128i,
}

impl FieldSerde for GF2_128 {
    #[inline(always)]
    fn serialize_into<W: std::io::Write>(&self, mut writer: W) {
        unsafe {
            writer
                .write_all(std::slice::from_raw_parts(
                    &self.v as *const __m128i as *const u8,
                    16,
                ))
                .unwrap(); // todo: error propagation
        }
    }

    #[inline(always)]
    fn serialized_size() -> usize {
        16
    }

    #[inline(always)]
    fn deserialize_from<R: std::io::Read>(mut reader: R) -> Self {
        let mut u = [0u8; 16];
        reader.read_exact(&mut u).unwrap(); // todo: error propagation
        unsafe {
            GF2_128 {
                v: *(u.as_ptr() as *const __m128i),
            }
        }
    }

    #[inline(always)]
    fn deserialize_from_ecc_format<R: std::io::Read>(mut _reader: R) -> Self {
        let mut u = [0u8; 32];
        _reader.read_exact(&mut u).unwrap(); // todo: error propagation
        unsafe {
            GF2_128 {
                v: *(u.as_ptr() as *const __m128i),
            }
        }
    }
}

impl Field for GF2_128 {
    const NAME: &'static str = "Galios Field 2^128";
    const SIZE: usize = 128 / 8;
    const FIELD_SIZE: usize = 128; // in bits

    const ZERO: Self = GF2_128 {
        v: unsafe { std::mem::zeroed() },
    };

    const INV_2: Self = GF2_128 {
        v: unsafe { std::mem::zeroed() },
    }; // should not be used

    #[inline(always)]
    fn zero() -> Self {
        GF2_128 {
            v: unsafe { std::mem::zeroed() },
        }
    }

    #[inline(always)]
    fn one() -> Self {
        GF2_128 {
            // 1 in the first bit
            v: unsafe { std::mem::transmute::<[i32; 4], __m128i>([1, 0, 0, 0]) }, // TODO check bit order
        }
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        let mut u = [0u8; 16];
        rng.fill_bytes(&mut u);
        unsafe {
            GF2_128 {
                v: *(u.as_ptr() as *const __m128i),
            }
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        GF2_128 {
            v: unsafe { std::mem::transmute::<[u32; 4], __m128i>([rng.next_u32() % 2, 0, 0, 0]) },
        }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        unsafe { std::mem::transmute::<__m128i, [u8; 16]>(self.v) == [0; 16] }
    }

    #[inline(always)]
    fn exp(&self, exponent: u128) -> Self {
        let mut e = exponent;
        let mut res = Self::one();
        let mut t = *self;
        while e > 0 {
            if e & 1 == 1 {
                res *= self;
            }
            t = t * t;
            e >>= 1;
        }
        res
    }

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        unimplemented!("inv is not implemented for GF2_128")
    }

    #[inline(always)]
    fn square(&self) -> Self {
        self * self
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        unimplemented!("u32 for GF128 doesn't make sense")
    }

    #[inline(always)]
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        unsafe {
            GF2_128 {
                v: *(bytes.as_ptr() as *const __m128i),
            }
        }
    }
}

impl BinomialExtensionField for GF2_128 {
    const DEGREE: usize = 128;
    const W: u32 = 0x87;

    type BaseField = GF2;

    #[inline(always)]
    fn mul_by_base_field(&self, base: &Self::BaseField) -> Self {
        if base.v == 0 {
            Self::zero()
        } else {
            *self
        }
    }

    #[inline(always)]
    fn add_by_base_field(&self, base: &Self::BaseField) -> Self {
        let mut res = *self;
        res.v = unsafe { _mm_xor_si128(res.v, _mm_set_epi64x(0, base.v as i64)) };
        res
    }

    #[inline(always)]
    fn first_base_field(&self) -> Self::BaseField {
        // but this doesn't make sense for GF2_128
        let v = unsafe { _mm_extract_epi64(self.v, 0) };
        GF2 { v: v as u8 }
    }
}

impl From<GF2> for GF2_128 {
    #[inline(always)]
    fn from(v: GF2) -> Self {
        GF2_128 {
            v: unsafe { _mm_set_epi64x(0, v.v as i64) },
        }
    }
}

impl Add for GF2_128 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        GF2_128 {
            v: unsafe { _mm_xor_si128(self.v, rhs.v) },
        }
    }
}

impl Sub for GF2_128 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        GF2_128 {
            v: unsafe { _mm_xor_si128(self.v, rhs.v) },
        }
    }
}

impl Add<&GF2_128> for GF2_128 {
    type Output = GF2_128;

    #[inline(always)]
    fn add(self, rhs: &GF2_128) -> GF2_128 {
        GF2_128 {
            v: unsafe { _mm_xor_si128(self.v, rhs.v) },
        }
    }
}

impl Sub<&GF2_128> for GF2_128 {
    type Output = GF2_128;

    #[inline(always)]
    fn sub(self, rhs: &GF2_128) -> GF2_128 {
        GF2_128 {
            v: unsafe { _mm_xor_si128(self.v, rhs.v) },
        }
    }
}

impl AddAssign for GF2_128 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.v = unsafe { _mm_xor_si128(self.v, rhs.v) };
    }
}

impl AddAssign<&GF2_128> for GF2_128 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &GF2_128) {
        self.v = unsafe { _mm_xor_si128(self.v, rhs.v) };
    }
}

impl SubAssign for GF2_128 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.v = unsafe { _mm_xor_si128(self.v, rhs.v) };
    }
}

impl SubAssign<&GF2_128> for GF2_128 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: &GF2_128) {
        self.v = unsafe { _mm_xor_si128(self.v, rhs.v) };
    }
}

unsafe fn gfmul(a: __m128i, b: __m128i, res: &mut __m128i) {
    let xmm_mask = _mm_setr_epi32((0xffffffff_u32) as i32, 0x0, 0x0, 0x0);

    // a = a0|a1, b = b0|b1

    let mut tmp3 = _mm_clmulepi64_si128(a, b, 0x00); // tmp3 = a0 * b0
    let mut tmp6 = _mm_clmulepi64_si128(a, b, 0x11); // tmp6 = a1 * b1

    let mut tmp4 = _mm_shuffle_epi32(a, 78); // tmp4 = a1|a0
    let mut tmp5 = _mm_shuffle_epi32(b, 78); // tmp5 = b1|b0
    tmp4 = _mm_xor_si128(tmp4, a); // tmp4 = (a0 + a1) | (a0 + a1)
    tmp5 = _mm_xor_si128(tmp5, b); // tmp5 = (b0 + b1) | (b0 + b1)

    tmp4 = _mm_clmulepi64_si128(tmp4, tmp5, 0x00); // tmp4 = (a0 + a1) * (b0 + b1)
    tmp4 = _mm_xor_si128(tmp4, tmp3); // tmp4 = (a0 + a1) * (b0 + b1) - a0 * b0
    tmp4 = _mm_xor_si128(tmp4, tmp6); // tmp4 = (a0 + a1) * (b0 + b1) - a0 * b0 - a1 * b1 = a0 * b1 + a1 * b0

    let tmp5_shifted_left = _mm_slli_si128(tmp4, 8);
    tmp4 = _mm_srli_si128(tmp4, 8);
    tmp3 = _mm_xor_si128(tmp3, tmp5_shifted_left);
    tmp6 = _mm_xor_si128(tmp6, tmp4);

    let mut tmp7 = _mm_srli_epi32(tmp6, 31);
    let mut tmp8 = _mm_srli_epi32(tmp6, 30);
    let tmp9 = _mm_srli_epi32(tmp6, 25);

    tmp7 = _mm_xor_si128(tmp7, tmp8);
    tmp7 = _mm_xor_si128(tmp7, tmp9);

    tmp8 = _mm_shuffle_epi32(tmp7, 147);
    tmp7 = _mm_and_si128(xmm_mask, tmp8);
    tmp8 = _mm_andnot_si128(xmm_mask, tmp8);

    tmp3 = _mm_xor_si128(tmp3, tmp8);
    tmp6 = _mm_xor_si128(tmp6, tmp7);

    let tmp10 = _mm_slli_epi32(tmp6, 1);
    tmp3 = _mm_xor_si128(tmp3, tmp10);

    let tmp11 = _mm_slli_epi32(tmp6, 2);
    tmp3 = _mm_xor_si128(tmp3, tmp11);

    let tmp12 = _mm_slli_epi32(tmp6, 7);
    tmp3 = _mm_xor_si128(tmp3, tmp12);

    *res = _mm_xor_si128(tmp3, tmp6);
}

impl Mul<GF2_128> for GF2_128 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self {
        let mut res = unsafe { std::mem::zeroed() };
        unsafe { gfmul(self.v, rhs.v, &mut res) };
        GF2_128 { v: res }
    }
}

impl Mul<&GF2_128> for GF2_128 {
    type Output = GF2_128;

    #[inline(always)]
    fn mul(self, rhs: &GF2_128) -> GF2_128 {
        let mut res = unsafe { std::mem::zeroed() };
        unsafe { gfmul(self.v, rhs.v, &mut res) };
        GF2_128 { v: res }
    }
}

impl Mul<GF2_128> for &GF2_128 {
    type Output = GF2_128;

    #[inline(always)]
    fn mul(self, rhs: GF2_128) -> GF2_128 {
        let mut res = unsafe { std::mem::zeroed() };
        unsafe { gfmul(self.v, rhs.v, &mut res) };
        GF2_128 { v: res }
    }
}

impl Mul<&GF2_128> for &GF2_128 {
    type Output = GF2_128;

    #[inline(always)]
    fn mul(self, rhs: &GF2_128) -> GF2_128 {
        let mut res = unsafe { std::mem::zeroed() };
        unsafe { gfmul(self.v, rhs.v, &mut res) };
        GF2_128 { v: res }
    }
}

impl MulAssign<GF2_128> for GF2_128 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: GF2_128) {
        let mut res = unsafe { std::mem::zeroed() };
        unsafe { gfmul(self.v, rhs.v, &mut res) };
        self.v = res;
    }
}

impl MulAssign<&GF2_128> for GF2_128 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: &GF2_128) {
        let mut res = unsafe { std::mem::zeroed() };
        unsafe { gfmul(self.v, rhs.v, &mut res) };
        self.v = res;
    }
}

impl Default for GF2_128 {
    #[inline(always)]
    fn default() -> Self {
        Self::zero()
    }
}

impl PartialEq for GF2_128 {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        unsafe { _mm_test_all_ones(_mm_cmpeq_epi8(self.v, other.v)) == 1 }
    }
}

impl Neg for GF2_128 {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self {
        self
    }
}

impl<T: std::borrow::Borrow<GF2_128>> std::iter::Sum<T> for GF2_128 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl<T: std::borrow::Borrow<GF2_128>> std::iter::Product<T> for GF2_128 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl From<u32> for GF2_128 {
    #[inline(always)]
    fn from(v: u32) -> Self {
        GF2_128 {
            v: unsafe { std::mem::transmute::<[u32; 4], __m128i>([v, 0, 0, 0]) },
        }
    }
}
