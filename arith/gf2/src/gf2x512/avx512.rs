use std::{
    arch::x86_64::*,
    fmt::Debug,
    mem::{transmute, zeroed},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::{Field, FieldSerde, FieldSerdeResult};

use crate::GF2;

#[derive(Clone, Copy)]
pub struct AVX512GF2x512 {
    pub v: __m512i,
}

impl FieldSerde for AVX512GF2x512 {
    const SERIALIZED_SIZE: usize = 64;

    #[inline(always)]
    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        unsafe {
            writer.write_all(transmute::<__m512i, [u8; Self::SERIALIZED_SIZE]>(self.v).as_ref())?
        };
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: std::io::Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let mut u = [0u8; Self::SERIALIZED_SIZE];
        reader.read_exact(&mut u)?;
        unsafe {
            Ok(AVX512GF2x512 {
                v: transmute::<[u8; Self::SERIALIZED_SIZE], __m512i>(u),
            })
        }
    }

    #[inline(always)]
    fn try_deserialize_from_ecc_format<R: std::io::Read>(mut _reader: R) -> FieldSerdeResult<Self> {
        unimplemented!()
    }
}

impl Field for AVX512GF2x512 {
    const NAME: &'static str = "AVX512 Galois Field 2 SIMD 512";

    const SIZE: usize = 512 / 8;

    const FIELD_SIZE: usize = 1; // in bits

    const ZERO: Self = AVX512GF2x512 {
        v: unsafe { zeroed() },
    };

    const ONE: Self = AVX512GF2x512 {
        v: unsafe { transmute::<[u64; 8], __m512i>([!0, !0, !0, !0, !0, !0, !0, !0]) },
    };

    const INV_2: Self = AVX512GF2x512 {
        v: unsafe { zeroed() },
    }; // should not be used

    #[inline(always)]
    fn zero() -> Self {
        AVX512GF2x512 {
            v: unsafe { zeroed() },
        }
    }

    #[inline(always)]
    fn one() -> Self {
        AVX512GF2x512 {
            v: unsafe { transmute::<[u64; 8], __m512i>([!0, !0, !0, !0, !0, !0, !0, !0]) },
        }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        unsafe {
            let zero = _mm512_setzero_si512();
            0xff == _mm512_cmpeq_epi64_mask(self.v, zero)
        }
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        let v = unsafe {
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
        };
        Self { v }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        let v = unsafe {
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
        };
        Self { v }
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

impl Default for AVX512GF2x512 {
    #[inline(always)]
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialEq for AVX512GF2x512 {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        unsafe { 0xff == _mm512_cmpeq_epi64_mask(self.v, other.v) }
    }
}

impl Mul<&AVX512GF2x512> for AVX512GF2x512 {
    type Output = AVX512GF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: &AVX512GF2x512) -> AVX512GF2x512 {
        AVX512GF2x512 {
            v: unsafe { _mm512_and_si512(self.v, rhs.v) },
        }
    }
}

impl Mul<AVX512GF2x512> for AVX512GF2x512 {
    type Output = AVX512GF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: AVX512GF2x512) -> AVX512GF2x512 {
        AVX512GF2x512 {
            v: unsafe { _mm512_and_si512(self.v, rhs.v) },
        }
    }
}

impl MulAssign<&AVX512GF2x512> for AVX512GF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn mul_assign(&mut self, rhs: &AVX512GF2x512) {
        self.v = unsafe { _mm512_and_si512(self.v, rhs.v) };
    }
}

impl MulAssign<AVX512GF2x512> for AVX512GF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn mul_assign(&mut self, rhs: AVX512GF2x512) {
        self.v = unsafe { _mm512_and_si512(self.v, rhs.v) };
    }
}

impl Sub for AVX512GF2x512 {
    type Output = AVX512GF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: AVX512GF2x512) -> AVX512GF2x512 {
        AVX512GF2x512 {
            v: unsafe { _mm512_xor_si512(self.v, rhs.v) },
        }
    }
}

impl SubAssign for AVX512GF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn sub_assign(&mut self, rhs: AVX512GF2x512) {
        self.v = unsafe { _mm512_xor_si512(self.v, rhs.v) };
    }
}

impl Add for AVX512GF2x512 {
    type Output = AVX512GF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: AVX512GF2x512) -> AVX512GF2x512 {
        AVX512GF2x512 {
            v: unsafe { _mm512_xor_si512(self.v, rhs.v) },
        }
    }
}

impl AddAssign for AVX512GF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: AVX512GF2x512) {
        self.v = unsafe { _mm512_xor_si512(self.v, rhs.v) };
    }
}

impl Add<&AVX512GF2x512> for AVX512GF2x512 {
    type Output = AVX512GF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: &AVX512GF2x512) -> AVX512GF2x512 {
        AVX512GF2x512 {
            v: unsafe { _mm512_xor_si512(self.v, rhs.v) },
        }
    }
}

impl AddAssign<&AVX512GF2x512> for AVX512GF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: &AVX512GF2x512) {
        self.v = unsafe { _mm512_xor_si512(self.v, rhs.v) };
    }
}

impl Sub<&AVX512GF2x512> for AVX512GF2x512 {
    type Output = AVX512GF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: &AVX512GF2x512) -> AVX512GF2x512 {
        AVX512GF2x512 {
            v: unsafe { _mm512_xor_si512(self.v, rhs.v) },
        }
    }
}

impl SubAssign<&AVX512GF2x512> for AVX512GF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn sub_assign(&mut self, rhs: &AVX512GF2x512) {
        self.v = unsafe { _mm512_xor_si512(self.v, rhs.v) };
    }
}

impl<T: std::borrow::Borrow<AVX512GF2x512>> std::iter::Sum<T> for AVX512GF2x512 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl<T: std::borrow::Borrow<AVX512GF2x512>> std::iter::Product<T> for AVX512GF2x512 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Neg for AVX512GF2x512 {
    type Output = AVX512GF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn neg(self) -> AVX512GF2x512 {
        AVX512GF2x512 { v: self.v }
    }
}

impl From<u32> for AVX512GF2x512 {
    #[inline(always)]
    fn from(v: u32) -> Self {
        assert!(v < 2);
        if v == 0 {
            AVX512GF2x512::ZERO
        } else {
            AVX512GF2x512::ONE
        }
    }
}

impl From<GF2> for AVX512GF2x512 {
    #[inline(always)]
    fn from(v: GF2) -> Self {
        assert!(v.v < 2);
        if v.v == 0 {
            AVX512GF2x512::ZERO
        } else {
            AVX512GF2x512::ONE
        }
    }
}

impl Debug for AVX512GF2x512 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut data = [0u8; 64];
        unsafe {
            _mm512_storeu_si512(data.as_mut_ptr() as *mut i32, self.v);
        }
        f.debug_struct("AVX512GF2x512")
            .field("data", &data)
            .finish()
    }
}
