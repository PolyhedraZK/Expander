use std::{
    arch::aarch64::*,
    fmt::Debug,
    mem::{transmute, zeroed},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::{Field, FieldSerde, FieldSerdeResult};

use crate::GF2;

#[derive(Clone, Copy, Debug)]
pub struct NeonGF2x512 {
    pub v: [uint32x4_t; 4],
}

impl FieldSerde for NeonGF2x512 {
    const SERIALIZED_SIZE: usize = 64;

    #[inline(always)]
    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        unsafe {
            writer.write_all(
                transmute::<[uint32x4_t; 4], [u8; Self::SERIALIZED_SIZE]>(self.v).as_ref(),
            )?
        };
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: std::io::Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let mut u = [0u8; Self::SERIALIZED_SIZE];
        reader.read_exact(&mut u)?;
        unsafe {
            Ok(NeonGF2x512 {
                v: transmute::<[u8; Self::SERIALIZED_SIZE], [uint32x4_t; 4]>(u),
            })
        }
    }

    #[inline(always)]
    fn try_deserialize_from_ecc_format<R: std::io::Read>(mut _reader: R) -> FieldSerdeResult<Self> {
        unimplemented!()
    }
}

impl Field for NeonGF2x512 {
    const NAME: &'static str = "Neon Galois Field 2 SIMD 512";

    const SIZE: usize = 512 / 8;

    const FIELD_SIZE: usize = 1; // in bits

    const ZERO: Self = NeonGF2x512 {
        v: unsafe { zeroed() },
    };

    const ONE: Self = NeonGF2x512 {
        v: unsafe { transmute::<[u64; 8], [uint32x4_t; 4]>([!0, !0, !0, !0, !0, !0, !0, !0]) },
    };

    const INV_2: Self = NeonGF2x512 {
        v: unsafe { zeroed() },
    }; // should not be used

    #[inline(always)]
    fn zero() -> Self {
        NeonGF2x512 {
            v: unsafe { zeroed() },
        }
    }

    #[inline(always)]
    fn one() -> Self {
        NeonGF2x512 {
            v: unsafe { transmute::<[u64; 8], [uint32x4_t; 4]>([!0, !0, !0, !0, !0, !0, !0, !0]) },
        }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.v
            .iter()
            .all(|vv| unsafe { transmute::<uint32x4_t, [u8; 16]>(*vv) == [0; 16] })
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        Self {
            v: [
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
            ],
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        Self {
            v: [
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
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

impl Default for NeonGF2x512 {
    #[inline(always)]
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialEq for NeonGF2x512 {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.v.iter().zip(other.v.iter()).all(|(a, b)| unsafe {
            transmute::<uint32x4_t, [u8; 16]>(*a) == transmute::<uint32x4_t, [u8; 16]>(*b)
        })
    }
}

impl Mul<&NeonGF2x512> for NeonGF2x512 {
    type Output = NeonGF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: &NeonGF2x512) -> NeonGF2x512 {
        NeonGF2x512 {
            v: unsafe {
                [
                    vandq_u32(self.v[0], rhs.v[0]),
                    vandq_u32(self.v[1], rhs.v[1]),
                    vandq_u32(self.v[2], rhs.v[2]),
                    vandq_u32(self.v[3], rhs.v[3]),
                ]
            },
        }
    }
}

impl Mul<NeonGF2x512> for NeonGF2x512 {
    type Output = NeonGF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: NeonGF2x512) -> NeonGF2x512 {
        NeonGF2x512 {
            v: unsafe {
                [
                    vandq_u32(self.v[0], rhs.v[0]),
                    vandq_u32(self.v[1], rhs.v[1]),
                    vandq_u32(self.v[2], rhs.v[2]),
                    vandq_u32(self.v[3], rhs.v[3]),
                ]
            },
        }
    }
}

impl MulAssign<&NeonGF2x512> for NeonGF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn mul_assign(&mut self, rhs: &NeonGF2x512) {
        self.v = unsafe {
            [
                vandq_u32(self.v[0], rhs.v[0]),
                vandq_u32(self.v[1], rhs.v[1]),
                vandq_u32(self.v[2], rhs.v[2]),
                vandq_u32(self.v[3], rhs.v[3]),
            ]
        };
    }
}

impl MulAssign<NeonGF2x512> for NeonGF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn mul_assign(&mut self, rhs: NeonGF2x512) {
        self.v = unsafe {
            [
                vandq_u32(self.v[0], rhs.v[0]),
                vandq_u32(self.v[1], rhs.v[1]),
                vandq_u32(self.v[2], rhs.v[2]),
                vandq_u32(self.v[3], rhs.v[3]),
            ]
        };
    }
}

impl Sub for NeonGF2x512 {
    type Output = NeonGF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: NeonGF2x512) -> NeonGF2x512 {
        NeonGF2x512 {
            v: unsafe {
                [
                    veorq_u32(self.v[0], rhs.v[0]),
                    veorq_u32(self.v[1], rhs.v[1]),
                    veorq_u32(self.v[2], rhs.v[2]),
                    veorq_u32(self.v[3], rhs.v[3]),
                ]
            },
        }
    }
}

impl SubAssign for NeonGF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn sub_assign(&mut self, rhs: NeonGF2x512) {
        self.v = unsafe {
            [
                veorq_u32(self.v[0], rhs.v[0]),
                veorq_u32(self.v[1], rhs.v[1]),
                veorq_u32(self.v[2], rhs.v[2]),
                veorq_u32(self.v[3], rhs.v[3]),
            ]
        };
    }
}

impl Add for NeonGF2x512 {
    type Output = NeonGF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: NeonGF2x512) -> NeonGF2x512 {
        NeonGF2x512 {
            v: unsafe {
                [
                    veorq_u32(self.v[0], rhs.v[0]),
                    veorq_u32(self.v[1], rhs.v[1]),
                    veorq_u32(self.v[2], rhs.v[2]),
                    veorq_u32(self.v[3], rhs.v[3]),
                ]
            },
        }
    }
}

impl AddAssign for NeonGF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: NeonGF2x512) {
        self.v = unsafe {
            [
                veorq_u32(self.v[0], rhs.v[0]),
                veorq_u32(self.v[1], rhs.v[1]),
                veorq_u32(self.v[2], rhs.v[2]),
                veorq_u32(self.v[3], rhs.v[3]),
            ]
        };
    }
}

impl Add<&NeonGF2x512> for NeonGF2x512 {
    type Output = NeonGF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: &NeonGF2x512) -> NeonGF2x512 {
        NeonGF2x512 {
            v: unsafe {
                [
                    veorq_u32(self.v[0], rhs.v[0]),
                    veorq_u32(self.v[1], rhs.v[1]),
                    veorq_u32(self.v[2], rhs.v[2]),
                    veorq_u32(self.v[3], rhs.v[3]),
                ]
            },
        }
    }
}

impl AddAssign<&NeonGF2x512> for NeonGF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: &NeonGF2x512) {
        self.v = unsafe {
            [
                veorq_u32(self.v[0], rhs.v[0]),
                veorq_u32(self.v[1], rhs.v[1]),
                veorq_u32(self.v[2], rhs.v[2]),
                veorq_u32(self.v[3], rhs.v[3]),
            ]
        };
    }
}

impl Sub<&NeonGF2x512> for NeonGF2x512 {
    type Output = NeonGF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: &NeonGF2x512) -> NeonGF2x512 {
        NeonGF2x512 {
            v: unsafe {
                [
                    veorq_u32(self.v[0], rhs.v[0]),
                    veorq_u32(self.v[1], rhs.v[1]),
                    veorq_u32(self.v[2], rhs.v[2]),
                    veorq_u32(self.v[3], rhs.v[3]),
                ]
            },
        }
    }
}

impl SubAssign<&NeonGF2x512> for NeonGF2x512 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn sub_assign(&mut self, rhs: &NeonGF2x512) {
        self.v = unsafe {
            [
                veorq_u32(self.v[0], rhs.v[0]),
                veorq_u32(self.v[1], rhs.v[1]),
                veorq_u32(self.v[2], rhs.v[2]),
                veorq_u32(self.v[3], rhs.v[3]),
            ]
        };
    }
}

impl<T: std::borrow::Borrow<NeonGF2x512>> std::iter::Sum<T> for NeonGF2x512 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl<T: std::borrow::Borrow<NeonGF2x512>> std::iter::Product<T> for NeonGF2x512 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Neg for NeonGF2x512 {
    type Output = NeonGF2x512;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn neg(self) -> NeonGF2x512 {
        NeonGF2x512 { v: self.v }
    }
}

impl From<u32> for NeonGF2x512 {
    #[inline(always)]
    fn from(v: u32) -> Self {
        assert!(v < 2);
        if v == 0 {
            NeonGF2x512::ZERO
        } else {
            NeonGF2x512::ONE
        }
    }
}

impl From<GF2> for NeonGF2x512 {
    #[inline(always)]
    fn from(v: GF2) -> Self {
        assert!(v.v < 2);
        if v.v == 0 {
            NeonGF2x512::ZERO
        } else {
            NeonGF2x512::ONE
        }
    }
}
