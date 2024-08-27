use crate::{field_common, GF2x8, GF2};

use crate::{
    BinomialExtensionField, Field, FieldSerde, FieldSerdeError, FieldSerdeResult, SimdField,
    GF2_128,
};
use std::fmt::Debug;
use std::{
    arch::x86_64::*,
    iter::{Product, Sum},
    mem::transmute,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

#[derive(Clone, Copy)]
pub struct AVX512GF2_128x8 {
    data: [__m512i; 2],
}

field_common!(AVX512GF2_128x8);

impl AVX512GF2_128x8 {
    #[inline(always)]
    pub(crate) fn pack_full(data: __m128i) -> [__m512i; 2] {
        [unsafe { _mm512_broadcast_i32x4(data) }, unsafe {
            _mm512_broadcast_i32x4(data)
        }]
    }
}

impl FieldSerde for AVX512GF2_128x8 {
    const SERIALIZED_SIZE: usize = 512 * 2 / 8;

    #[inline(always)]
    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        unsafe {
            let mut data = [0u8; 128];
            _mm512_storeu_si512(data.as_mut_ptr() as *mut i32, self.data[0]);
            _mm512_storeu_si512((data.as_mut_ptr() as *mut i32).offset(16), self.data[1]);
            writer.write_all(&data)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: std::io::Read>(
        mut reader: R,
    ) -> Result<AVX512GF2_128x8, FieldSerdeError> {
        let mut data = [0u8; Self::SERIALIZED_SIZE];
        reader.read_exact(&mut data).unwrap();
        unsafe {
            Ok(Self {
                data: [
                    _mm512_loadu_si512(data.as_ptr() as *const i32),
                    _mm512_loadu_si512((data.as_ptr() as *const i32).offset(16)),
                ],
            })
        }
    }

    #[inline(always)]
    fn try_deserialize_from_ecc_format<R: std::io::Read>(mut _reader: R) -> FieldSerdeResult<Self> {
        unimplemented!("We don't have a serialization for gf2_128 in ecc yet.")

        // let mut buf = [0u8; 32];
        // reader.read_exact(&mut buf)?;
        // let data: __m128i = unsafe { _mm_loadu_si128(buf.as_ptr() as *const __m128i) };
        // Ok(Self {
        //     data: Self::pack_full(data),
        // })
    }
}

const PACKED_0: [__m512i; 2] = [unsafe { transmute([0; 16]) }, unsafe { transmute([0; 16]) }];
const _M512_INV_2: __m512i = unsafe {
    transmute([
        67_u64,
        (1_u64) << 63,
        67_u64,
        (1_u64) << 63,
        67_u64,
        (1_u64) << 63,
        67_u64,
        (1_u64) << 63,
    ])
};
const PACKED_INV_2: [__m512i; 2] = [_M512_INV_2, _M512_INV_2]; // Should not be used?

// p(x) = x^128 + x^7 + x^2 + x + 1
impl Field for AVX512GF2_128x8 {
    const NAME: &'static str = "AVX512 Galios Field 2^128";

    // size in bytes
    const SIZE: usize = 512 * 2 / 8;

    const ZERO: Self = Self { data: PACKED_0 };

    const INV_2: Self = Self { data: PACKED_INV_2 };

    const FIELD_SIZE: usize = 128;

    #[inline(always)]
    fn zero() -> Self {
        unsafe {
            let zero = _mm512_setzero_si512();
            Self { data: [zero, zero] }
        }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        unsafe {
            let zero = _mm512_setzero_si512();
            let cmp_0 = _mm512_cmpeq_epi64_mask(self.data[0], zero);
            let cmp_1 = _mm512_cmpeq_epi64_mask(self.data[1], zero);
            (cmp_0 & cmp_1) == 0xFF // All 16 64-bit integers are equal (zero)
        }
    }

    #[inline(always)]
    fn one() -> Self {
        unsafe {
            let one = _mm512_set_epi64(0, 1, 0, 1, 0, 1, 0, 1);
            Self { data: [one, one] }
        }
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        let data = [
            unsafe {
                _mm512_set_epi64(
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                )
            },
            unsafe {
                _mm512_set_epi64(
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                )
            },
        ];
        Self { data }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        let data = [
            unsafe {
                _mm512_set_epi64(
                    0,
                    (rng.next_u64() % 2) as i64,
                    0,
                    (rng.next_u64() % 2) as i64,
                    0,
                    (rng.next_u64() % 2) as i64,
                    0,
                    (rng.next_u64() % 2) as i64,
                )
            },
            unsafe {
                _mm512_set_epi64(
                    0,
                    (rng.next_u64() % 2) as i64,
                    0,
                    (rng.next_u64() % 2) as i64,
                    0,
                    (rng.next_u64() % 2) as i64,
                    0,
                    (rng.next_u64() % 2) as i64,
                )
            },
        ];
        Self { data }
    }

    #[inline(always)]
    fn exp(&self, exponent: u128) -> Self {
        let mut e = exponent;
        let mut res = Self::one();
        let mut t = *self;
        while e != 0 {
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
        if self.is_zero() {
            return None;
        }
        let p_m2 = !(0u128) - 1;
        Some(Self::exp(self, p_m2))
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        unimplemented!("self is a vector, cannot convert to u32")
    }

    #[inline(always)]
    fn from_uniform_bytes(_bytes: &[u8; 32]) -> Self {
        todo!()
    }

    #[inline(always)]
    fn square(&self) -> Self {
        *self * *self
    }

    #[inline(always)]
    fn double(&self) -> Self {
        Self::ZERO
    }

    #[inline(always)]
    fn mul_by_2(&self) -> Self {
        Self::ZERO
    }

    #[inline(always)]
    fn mul_by_3(&self) -> Self {
        *self
    }

    #[inline(always)]
    fn mul_by_5(&self) -> Self {
        *self
    }

    #[inline(always)]
    fn mul_by_6(&self) -> Self {
        Self::ZERO
    }
}
/*
credit to intel for the original implementation
void gfmul(__m128i a, __m128i b, __m128i *res) {
    __m128i tmp0, tmp1, tmp2, tmp3, tmp4, tmp5, tmp6;
    __m128i tmp7, tmp8, tmp9, tmp10, tmp11, tmp12;
    __m128i XMMMASK = _mm_setr_epi32(0xffffffff, 0x0, 0x0, 0x0);

    // a = a0|a1, b = b0|b1

    tmp3 = _mm_clmulepi64_si128(a, b, 0x00); // tmp3 = a0 * b0
    tmp6 = _mm_clmulepi64_si128(a, b, 0x11); // tmp6 = a1 * b1

    tmp4 = _mm_shuffle_epi32(a, 78); // tmp4 = a1|a0
    tmp5 = _mm_shuffle_epi32(b, 78); // tmp5 = b1|b0
    tmp4 = _mm_xor_si128(tmp4, a); // tmp4 = (a0 + a1) | (a0 + a1)
    tmp5 = _mm_xor_si128(tmp5, b); // tmp5 = (b0 + b1) | (b0 + b1)

    tmp4 = _mm_clmulepi64_si128(tmp4, tmp5, 0x00); // tmp4 = (a0 + a1) * (b0 + b1)
    tmp4 = _mm_xor_si128(tmp4, tmp3); // tmp4 = (a0 + a1) * (b0 + b1) - a0 * b0
    tmp4 = _mm_xor_si128(tmp4, tmp6); // tmp4 = (a0 + a1) * (b0 + b1) - a0 * b0 - a1 * b1 = a0 * b1 + a1 * b0

    tmp5 = _mm_slli_si128(tmp4, 8);
    tmp4 = _mm_srli_si128(tmp4, 8);
    tmp3 = _mm_xor_si128(tmp3, tmp5);
    tmp6 = _mm_xor_si128(tmp6, tmp4);

    tmp7 = _mm_srli_epi32(tmp6, 31);
    tmp8 = _mm_srli_epi32(tmp6, 30);
    tmp9 = _mm_srli_epi32(tmp6, 25);

    tmp7 = _mm_xor_si128(tmp7, tmp8);
    tmp7 = _mm_xor_si128(tmp7, tmp9);

    tmp8 = _mm_shuffle_epi32(tmp7, 147);
    tmp7 = _mm_and_si128(XMMMASK, tmp8);
    tmp8 = _mm_andnot_si128(XMMMASK, tmp8);

    tmp3 = _mm_xor_si128(tmp3, tmp8);
    tmp6 = _mm_xor_si128(tmp6, tmp7);

    tmp10 = _mm_slli_epi32(tmp6, 1);
    tmp3 = _mm_xor_si128(tmp3, tmp10);

    tmp11 = _mm_slli_epi32(tmp6, 2);
    tmp3 = _mm_xor_si128(tmp3, tmp11);

    tmp12 = _mm_slli_epi32(tmp6, 7);
    tmp3 = _mm_xor_si128(tmp3, tmp12);

    *res = _mm_xor_si128(tmp3, tmp6);
}

*/

/*
AVX 512 version
void gfmul_avx512(__m512i a, __m512i b, __m512i *res) {
    __m512i tmp0, tmp1, tmp2, tmp3, tmp4, tmp5, tmp6;
    __m512i tmp7, tmp8, tmp9, tmp10, tmp11, tmp12;
    __m512i XMMMASK = _mm512_set_epi32(
        0, 0, 0, 0xffffffff,
        0, 0, 0, 0xffffffff,
        0, 0, 0, 0xffffffff,
        0, 0, 0, 0xffffffff
    );

    tmp3 = _mm512_clmulepi64_epi128(a, b, 0x00);
    tmp6 = _mm512_clmulepi64_epi128(a, b, 0x11);

    tmp4 = _mm512_shuffle_epi32(a, _MM_PERM_BADC);
    tmp5 = _mm512_shuffle_epi32(b, _MM_PERM_BADC);
    tmp4 = _mm512_xor_si512(tmp4, a);
    tmp5 = _mm512_xor_si512(tmp5, b);

    tmp4 = _mm512_clmulepi64_epi128(tmp4, tmp5, 0x00);
    tmp4 = _mm512_xor_si512(tmp4, tmp3);
    tmp4 = _mm512_xor_si512(tmp4, tmp6);

    tmp5 = _mm512_bslli_epi128(tmp4, 8);
    tmp4 = _mm512_bsrli_epi128(tmp4, 8);
    tmp3 = _mm512_xor_si512(tmp3, tmp5);
    tmp6 = _mm512_xor_si512(tmp6, tmp4);

    tmp7 = _mm512_srli_epi32(tmp6, 31);
    tmp8 = _mm512_srli_epi32(tmp6, 30);
    tmp9 = _mm512_srli_epi32(tmp6, 25);

    tmp7 = _mm512_xor_si512(tmp7, tmp8);
    tmp7 = _mm512_xor_si512(tmp7, tmp9);

    tmp8 = _mm512_shuffle_epi32(tmp7, _MM_PERM_ABCD);
    tmp7 = _mm512_and_si512(XMMMASK, tmp8);
    tmp8 = _mm512_andnot_si512(XMMMASK, tmp8);

    tmp3 = _mm512_xor_si512(tmp3, tmp8);
    tmp6 = _mm512_xor_si512(tmp6, tmp7);

    tmp10 = _mm512_slli_epi32(tmp6, 1);
    tmp3 = _mm512_xor_si512(tmp3, tmp10);

    tmp11 = _mm512_slli_epi32(tmp6, 2);
    tmp3 = _mm512_xor_si512(tmp3, tmp11);

    tmp12 = _mm512_slli_epi32(tmp6, 7);
    tmp3 = _mm512_xor_si512(tmp3, tmp12);

    *res = _mm512_xor_si512(tmp3, tmp6);
}
 */

impl From<u32> for AVX512GF2_128x8 {
    #[inline(always)]
    fn from(v: u32) -> AVX512GF2_128x8 {
        assert!(v < 2); // only 0 and 1 are allowed
        let data = unsafe { _mm512_set_epi64(0, v as i64, 0, v as i64, 0, v as i64, 0, v as i64) };
        AVX512GF2_128x8 { data: [data, data] }
    }
}

impl Neg for AVX512GF2_128x8 {
    type Output = AVX512GF2_128x8;

    #[inline(always)]
    fn neg(self) -> AVX512GF2_128x8 {
        self
    }
}

impl Debug for AVX512GF2_128x8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut data = [0u8; 128];
        unsafe {
            _mm512_storeu_si512(data.as_mut_ptr() as *mut i32, self.data[0]);
            _mm512_storeu_si512((data.as_mut_ptr() as *mut i32).offset(16), self.data[0]);
        }
        f.debug_struct("AVX512GF2_128x8")
            .field("data", &data)
            .finish()
    }
}

impl PartialEq for AVX512GF2_128x8 {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            let cmp_0 = _mm512_cmpeq_epi64_mask(self.data[0], other.data[0]);
            let cmp_1 = _mm512_cmpeq_epi64_mask(self.data[1], other.data[1]);
            (cmp_0 & cmp_1) == 0xFF // All 16 64-bit integers are equal
        }
    }
}

impl Default for AVX512GF2_128x8 {
    #[inline(always)]
    fn default() -> Self {
        Self::zero()
    }
}

impl From<GF2_128> for AVX512GF2_128x8 {
    #[inline(always)]
    fn from(v: GF2_128) -> AVX512GF2_128x8 {
        AVX512GF2_128x8 {
            data: Self::pack_full(v.v),
        }
    }
}

impl SimdField for AVX512GF2_128x8 {
    #[inline(always)]
    fn scale(&self, challenge: &Self::Scalar) -> Self {
        let simd_challenge = AVX512GF2_128x8::from(*challenge);
        *self * simd_challenge
    }
    type Scalar = GF2_128;

    #[inline(always)]
    fn pack_size() -> usize {
        8
    }
}

#[inline(always)]
fn add_internal(a: &AVX512GF2_128x8, b: &AVX512GF2_128x8) -> AVX512GF2_128x8 {
    unsafe {
        AVX512GF2_128x8 {
            data: [
                _mm512_xor_si512(a.data[0], b.data[0]),
                _mm512_xor_si512(a.data[1], b.data[1]),
            ],
        }
    }
}

#[inline(always)]
fn sub_internal(a: &AVX512GF2_128x8, b: &AVX512GF2_128x8) -> AVX512GF2_128x8 {
    unsafe {
        AVX512GF2_128x8 {
            data: [
                _mm512_xor_si512(a.data[0], b.data[0]),
                _mm512_xor_si512(a.data[1], b.data[1]),
            ],
        }
    }
}

#[inline]
fn _m512_mul_internal(a: __m512i, b: __m512i) -> __m512i {
    unsafe {
        let xmmmask = _mm512_set_epi32(
            0,
            0,
            0,
            0xffffffffu32 as i32,
            0,
            0,
            0,
            0xffffffffu32 as i32,
            0,
            0,
            0,
            0xffffffffu32 as i32,
            0,
            0,
            0,
            0xffffffffu32 as i32,
        );

        let mut tmp3 = _mm512_clmulepi64_epi128(a, b, 0x00);
        let mut tmp6 = _mm512_clmulepi64_epi128(a, b, 0x11);

        let mut tmp4 = _mm512_shuffle_epi32(a, _MM_PERM_BADC);
        let mut tmp5 = _mm512_shuffle_epi32(b, _MM_PERM_BADC);
        tmp4 = _mm512_xor_si512(tmp4, a);
        tmp5 = _mm512_xor_si512(tmp5, b);

        tmp4 = _mm512_clmulepi64_epi128(tmp4, tmp5, 0x00);
        tmp4 = _mm512_xor_si512(tmp4, tmp3);
        tmp4 = _mm512_xor_si512(tmp4, tmp6);

        tmp5 = _mm512_bslli_epi128(tmp4, 8);
        tmp4 = _mm512_bsrli_epi128(tmp4, 8);
        tmp3 = _mm512_xor_si512(tmp3, tmp5);
        tmp6 = _mm512_xor_si512(tmp6, tmp4);

        let tmp7 = _mm512_srli_epi32(tmp6, 31);
        let tmp8 = _mm512_srli_epi32(tmp6, 30);
        let tmp9 = _mm512_srli_epi32(tmp6, 25);

        let mut tmp7 = _mm512_xor_si512(tmp7, tmp8);
        tmp7 = _mm512_xor_si512(tmp7, tmp9);

        let mut tmp8 = _mm512_shuffle_epi32(tmp7, _MM_PERM_CBAD);
        tmp7 = _mm512_and_si512(xmmmask, tmp8);
        tmp8 = _mm512_andnot_si512(xmmmask, tmp8);

        tmp3 = _mm512_xor_si512(tmp3, tmp8);
        tmp6 = _mm512_xor_si512(tmp6, tmp7);

        let tmp10 = _mm512_slli_epi32(tmp6, 1);
        tmp3 = _mm512_xor_si512(tmp3, tmp10);

        let tmp11 = _mm512_slli_epi32(tmp6, 2);
        tmp3 = _mm512_xor_si512(tmp3, tmp11);

        let tmp12 = _mm512_slli_epi32(tmp6, 7);
        tmp3 = _mm512_xor_si512(tmp3, tmp12);

        let result = _mm512_xor_si512(tmp3, tmp6);

        result
    }
}

#[inline(always)]
fn mul_internal(a: &AVX512GF2_128x8, b: &AVX512GF2_128x8) -> AVX512GF2_128x8 {
    AVX512GF2_128x8 {
        data: [
            _m512_mul_internal(a.data[0], b.data[0]),
            _m512_mul_internal(a.data[1], b.data[1]),
        ],
    }
}

// abcdefgh -> aacceegg
#[inline(always)]
pub fn duplicate_even_bits(byte: u8) -> u8 {
    let even_bits = byte & 0b10101010;
    let even_bits_shifted = even_bits >> 1;
    even_bits | even_bits_shifted
}

// abcdefgh -> bbddffhh
#[inline(always)]
pub fn duplicate_odd_bits(byte: u8) -> u8 {
    let odd_bits = byte & 0b01010101;
    let odd_bits_shifted = odd_bits << 1;
    odd_bits | odd_bits_shifted
}

impl BinomialExtensionField for AVX512GF2_128x8 {
    const DEGREE: usize = 128;
    const W: u32 = 0x87;

    type BaseField = GF2x8;

    #[inline(always)]
    fn mul_by_base_field(&self, base: &Self::BaseField) -> Self {
        let mask_even = duplicate_even_bits(base.v);
        let mask_odd = duplicate_odd_bits(base.v);

        Self {
            data: [
                unsafe { _mm512_maskz_mov_epi64(mask_even, self.data[0]) },
                unsafe { _mm512_maskz_mov_epi64(mask_odd, self.data[1]) },
            ],
        }
    }

    #[inline(always)]
    fn add_by_base_field(&self, base: &Self::BaseField) -> Self {
        let v0 = ((base.v >> 7) & 1u8) as i64;
        let v1 = ((base.v >> 6) & 1u8) as i64;
        let v2 = ((base.v >> 5) & 1u8) as i64;
        let v3 = ((base.v >> 4) & 1u8) as i64;
        let v4 = ((base.v >> 3) & 1u8) as i64;
        let v5 = ((base.v >> 2) & 1u8) as i64;
        let v6 = ((base.v >> 1) & 1u8) as i64;
        let v7 = ((base.v >> 0) & 1u8) as i64;

        let mut res = *self;
        res.data[0] =
            unsafe { _mm512_xor_si512(res.data[0], _mm512_set_epi64(0, v0, 0, v2, 0, v4, 0, v6)) };
        res.data[1] =
            unsafe { _mm512_xor_si512(res.data[1], _mm512_set_epi64(0, v1, 0, v3, 0, v5, 0, v7)) };

        res
    }
}

impl From<GF2x8> for AVX512GF2_128x8 {
    #[inline(always)]
    fn from(v: GF2x8) -> Self {
        let v0 = ((v.v >> 7) & 1u8) as i64;
        let v1 = ((v.v >> 6) & 1u8) as i64;
        let v2 = ((v.v >> 5) & 1u8) as i64;
        let v3 = ((v.v >> 4) & 1u8) as i64;
        let v4 = ((v.v >> 3) & 1u8) as i64;
        let v5 = ((v.v >> 2) & 1u8) as i64;
        let v6 = ((v.v >> 1) & 1u8) as i64;
        let v7 = ((v.v >> 0) & 1u8) as i64;

        AVX512GF2_128x8 {
            data: [
                unsafe { _mm512_set_epi64(0, v0, 0, v2, 0, v4, 0, v6) }, // even
                unsafe { _mm512_set_epi64(0, v1, 0, v3, 0, v5, 0, v7) }, // odd
            ],
        }
    }
}

impl Mul<GF2> for AVX512GF2_128x8 {
    type Output = AVX512GF2_128x8;

    #[inline(always)]
    fn mul(self, rhs: GF2) -> Self::Output {
        if rhs.is_zero() {
            Self::zero()
        } else {
            self
        }
    }
}

impl Add<GF2> for AVX512GF2_128x8 {
    type Output = AVX512GF2_128x8;
    #[inline(always)]
    fn add(self, rhs: GF2) -> Self::Output {
        let rhs_extended = unsafe { _mm512_maskz_set1_epi64(0b01010101, rhs.v as i64) };
        AVX512GF2_128x8 {
            data: [
                unsafe { _mm512_xor_si512(self.data[0], rhs_extended) },
                unsafe { _mm512_xor_si512(self.data[1], rhs_extended) },
            ],
        }
    }
}
