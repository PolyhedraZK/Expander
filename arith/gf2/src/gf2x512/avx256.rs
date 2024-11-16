use std::{
    arch::x86_64::*,
    fmt::Debug,
    mem::{transmute, zeroed},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::{Field, FieldSerde, FieldSerdeResult};

use crate::GF2;

#[derive(Clone, Copy)]
pub struct AVX256GF2x512 {
    pub v: [__m256i; 2],
}

impl FieldSerde for AVX256GF2x512 {
    const SERIALIZED_SIZE: usize = 64;

    #[inline(always)]
    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        unsafe {
            writer.write_all(
                transmute::<[__m256i; 2], [u8; Self::SERIALIZED_SIZE]>(self.v).as_ref(),
            )?
        };
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: std::io::Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let mut u = [0u8; Self::SERIALIZED_SIZE];
        reader.read_exact(&mut u)?;
        unsafe {
            Ok(AVX256GF2x512 {
                v: transmute::<[u8; Self::SERIALIZED_SIZE], [__m256i; 2]>(u),
            })
        }
    }

    #[inline(always)]
    fn try_deserialize_from_ecc_format<R: std::io::Read>(mut _reader: R) -> FieldSerdeResult<Self> {
        unimplemented!()
    }
}

impl Field for AVX256GF2x512 {
    const NAME: &'static str = "AVX256 Galois Field 2 SIMD 512";

    const SIZE: usize = 512 / 8;

    const FIELD_SIZE: usize = 1; // in bits

    const ZERO: Self = AVX256GF2x512 {
        v: unsafe { zeroed() },
    };

    const ONE: Self = AVX256GF2x512 {
        v: unsafe { transmute::<[u64; 8], [__m256i; 2]>([!0, !0, !0, !0, !0, !0, !0, !0]) },
    };

    const INV_2: Self = AVX256GF2x512 {
        v: unsafe { zeroed() },
    }; // should not be used

    #[inline(always)]
    fn zero() -> Self {
        AVX256GF2x512 {
            v: unsafe { zeroed() },
        }
    }

    #[inline(always)]
    fn one() -> Self {
        AVX256GF2x512 {
            v: unsafe { transmute::<[u64; 8], [__m256i; 2]>([!0, !0, !0, !0, !0, !0, !0, !0]) },
        }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        unsafe {
            let zero = _mm256_setzero_si256();
            let cmp0 = _mm256_movemask_epi8(_mm256_cmpeq_epi8(self.v[0], zero));
            let cmp1 = _mm256_movemask_epi8(_mm256_cmpeq_epi8(self.v[1], zero));
            !0 == (cmp0 & cmp1)
        }
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        Self {
            v: [
                unsafe {
                    _mm256_set_epi64x(
                        rng.next_u64() as i64,
                        rng.next_u64() as i64,
                        rng.next_u64() as i64,
                        rng.next_u64() as i64,
                    )
                },
                unsafe {
                    _mm256_set_epi64x(
                        rng.next_u64() as i64,
                        rng.next_u64() as i64,
                        rng.next_u64() as i64,
                        rng.next_u64() as i64,
                    )
                },
            ],
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        Self {
            v: [
                unsafe {
                    _mm256_set_epi64x(
                        rng.next_u64() as i64,
                        rng.next_u64() as i64,
                        rng.next_u64() as i64,
                        rng.next_u64() as i64,
                    )
                },
                unsafe {
                    _mm256_set_epi64x(
                        rng.next_u64() as i64,
                        rng.next_u64() as i64,
                        rng.next_u64() as i64,
                        rng.next_u64() as i64,
                    )
                },
            ],
        }
    }

    #[inline(always)]
    fn exp(&self, exponent: u128) -> Self {
        if exponent == 0 {
            return Self::one();
        }
        *self
    }

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        unimplemented!()
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        unimplemented!("u32 for GF2x512 doesn't make sense")
    }

    #[inline(always)]
    fn from_uniform_bytes(_bytes: &[u8; 32]) -> Self {
        unimplemented!("from uniformly random [u8; 32] does not make sense for GF2x512")
    }
}

impl Default for AVX256GF2x512 {
    #[inline(always)]
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialEq for AVX256GF2x512 {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            let cmp0 = _mm256_movemask_epi8(_mm256_cmpeq_epi8(self.v[0], other.v[0]));
            let cmp1 = _mm256_movemask_epi8(_mm256_cmpeq_epi8(self.v[1], other.v[1]));

            !0 == (cmp0 & cmp1)
        }
    }
}

impl Mul<&AVX256GF2x512> for AVX256GF2x512 {
    type Output = AVX256GF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: &AVX256GF2x512) -> AVX256GF2x512 {
        AVX256GF2x512 {
            v: unsafe {
                [
                    _mm256_and_si256(self.v[0], rhs.v[0]),
                    _mm256_and_si256(self.v[1], rhs.v[1]),
                ]
            },
        }
    }
}

impl Mul<AVX256GF2x512> for AVX256GF2x512 {
    type Output = AVX256GF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: AVX256GF2x512) -> AVX256GF2x512 {
        AVX256GF2x512 {
            v: unsafe {
                [
                    _mm256_and_si256(self.v[0], rhs.v[0]),
                    _mm256_and_si256(self.v[1], rhs.v[1]),
                ]
            },
        }
    }
}

impl MulAssign<&AVX256GF2x512> for AVX256GF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn mul_assign(&mut self, rhs: &AVX256GF2x512) {
        self.v = unsafe {
            [
                _mm256_and_si256(self.v[0], rhs.v[0]),
                _mm256_and_si256(self.v[1], rhs.v[1]),
            ]
        };
    }
}

impl MulAssign<AVX256GF2x512> for AVX256GF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn mul_assign(&mut self, rhs: AVX256GF2x512) {
        self.v = unsafe {
            [
                _mm256_and_si256(self.v[0], rhs.v[0]),
                _mm256_and_si256(self.v[1], rhs.v[1]),
            ]
        };
    }
}

impl Sub for AVX256GF2x512 {
    type Output = AVX256GF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: AVX256GF2x512) -> AVX256GF2x512 {
        AVX256GF2x512 {
            v: unsafe {
                [
                    _mm256_xor_si256(self.v[0], rhs.v[0]),
                    _mm256_xor_si256(self.v[1], rhs.v[1]),
                ]
            },
        }
    }
}

impl SubAssign for AVX256GF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn sub_assign(&mut self, rhs: AVX256GF2x512) {
        self.v = unsafe {
            [
                _mm256_xor_si256(self.v[0], rhs.v[0]),
                _mm256_xor_si256(self.v[1], rhs.v[1]),
            ]
        };
    }
}

impl Add for AVX256GF2x512 {
    type Output = AVX256GF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: AVX256GF2x512) -> AVX256GF2x512 {
        AVX256GF2x512 {
            v: unsafe {
                [
                    _mm256_xor_si256(self.v[0], rhs.v[0]),
                    _mm256_xor_si256(self.v[1], rhs.v[1]),
                ]
            },
        }
    }
}

impl AddAssign for AVX256GF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: AVX256GF2x512) {
        self.v = unsafe {
            [
                _mm256_xor_si256(self.v[0], rhs.v[0]),
                _mm256_xor_si256(self.v[1], rhs.v[1]),
            ]
        };
    }
}

impl Add<&AVX256GF2x512> for AVX256GF2x512 {
    type Output = AVX256GF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: &AVX256GF2x512) -> AVX256GF2x512 {
        AVX256GF2x512 {
            v: unsafe {
                [
                    _mm256_xor_si256(self.v[0], rhs.v[0]),
                    _mm256_xor_si256(self.v[1], rhs.v[1]),
                ]
            },
        }
    }
}

impl AddAssign<&AVX256GF2x512> for AVX256GF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: &AVX256GF2x512) {
        self.v = unsafe {
            [
                _mm256_xor_si256(self.v[0], rhs.v[0]),
                _mm256_xor_si256(self.v[1], rhs.v[1]),
            ]
        };
    }
}

impl Sub<&AVX256GF2x512> for AVX256GF2x512 {
    type Output = AVX256GF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: &AVX256GF2x512) -> AVX256GF2x512 {
        AVX256GF2x512 {
            v: unsafe {
                [
                    _mm256_xor_si256(self.v[0], rhs.v[0]),
                    _mm256_xor_si256(self.v[1], rhs.v[1]),
                ]
            },
        }
    }
}

impl SubAssign<&AVX256GF2x512> for AVX256GF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn sub_assign(&mut self, rhs: &AVX256GF2x512) {
        self.v = unsafe {
            [
                _mm256_xor_si256(self.v[0], rhs.v[0]),
                _mm256_xor_si256(self.v[1], rhs.v[1]),
            ]
        };
    }
}

impl<T: std::borrow::Borrow<AVX256GF2x512>> std::iter::Sum<T> for AVX256GF2x512 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl<T: std::borrow::Borrow<AVX256GF2x512>> std::iter::Product<T> for AVX256GF2x512 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Neg for AVX256GF2x512 {
    type Output = AVX256GF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn neg(self) -> AVX256GF2x512 {
        AVX256GF2x512 { v: self.v }
    }
}

impl From<u32> for AVX256GF2x512 {
    #[inline(always)]
    fn from(v: u32) -> Self {
        assert!(v < 2);
        if v == 0 {
            AVX256GF2x512::ZERO
        } else {
            AVX256GF2x512::ONE
        }
    }
}

impl From<GF2> for AVX256GF2x512 {
    #[inline(always)]
    fn from(v: GF2) -> Self {
        assert!(v.v < 2);
        if v.v == 0 {
            AVX256GF2x512::ZERO
        } else {
            AVX256GF2x512::ONE
        }
    }
}

impl Debug for AVX256GF2x512 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut data = [0u8; 64];
        unsafe {
            _mm256_storeu_si256(data.as_mut_ptr() as *mut __m256i, self.v[0]);
            _mm256_storeu_si256((data.as_mut_ptr() as *mut __m256i).offset(1), self.v[1]);
        }
        f.debug_struct("AVX256GF2x512")
            .field("data", &data)
            .finish()
    }
}
