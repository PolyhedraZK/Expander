use std::{
    hash::{Hash, Hasher},
    iter::{Product, Sum},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::{ExtensionField, Field, SimdField};

use ethnum::U256;
use rand::RngCore;
use serdes::{ExpSerde, SerdeError};

use crate::Goldilocksx8;

/// Degree-2 extension of Goldilocks field with 8-element SIMD operations
/// Represents elements as a + bX where X^2 = 7
#[derive(Copy, Clone, Debug, Default, PartialOrd, Ord)]
pub struct GoldilocksExt2x8 {
    pub c0: Goldilocksx8, // constant term
    pub c1: Goldilocksx8, // coefficient of X
}

impl ExpSerde for GoldilocksExt2x8 {
    const SERIALIZED_SIZE: usize = 32;

    fn serialize_into<W>(&self, mut writer: W) -> Result<(), SerdeError>
    where
        W: std::io::Write,
    {
        self.c0.serialize_into(&mut writer)?;
        self.c1.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R>(mut reader: R) -> Result<Self, SerdeError>
    where
        R: std::io::Read,
    {
        let c0 = Goldilocksx8::deserialize_from(&mut reader)?;
        let c1 = Goldilocksx8::deserialize_from(&mut reader)?;
        Ok(Self { c0, c1 })
    }
}

impl GoldilocksExt2x8 {
    pub fn new(c0: Goldilocksx8, c1: Goldilocksx8) -> Self {
        Self { c0, c1 }
    }
}

impl PartialEq for GoldilocksExt2x8 {
    fn eq(&self, other: &Self) -> bool {
        self.c0 == other.c0 && self.c1 == other.c1
    }
}

impl Eq for GoldilocksExt2x8 {}

impl<'a> Add<&'a GoldilocksExt2x8> for GoldilocksExt2x8 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: &'a Self) -> Self::Output {
        Self {
            c0: self.c0 + rhs.c0,
            c1: self.c1 + rhs.c1,
        }
    }
}

impl Add for GoldilocksExt2x8 {
    type Output = Self;

    #[inline]
    #[allow(clippy::op_ref)]
    fn add(self, rhs: Self) -> Self::Output {
        self + &rhs
    }
}

impl<'a> AddAssign<&'a GoldilocksExt2x8> for GoldilocksExt2x8 {
    #[inline]
    fn add_assign(&mut self, rhs: &'a Self) {
        *self = *self + rhs;
    }
}

impl AddAssign for GoldilocksExt2x8 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl<'a> Sum<&'a GoldilocksExt2x8> for GoldilocksExt2x8 {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, x| acc + x)
    }
}

impl Sum for GoldilocksExt2x8 {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, x| acc + x)
    }
}

impl<'a> Sub<&'a GoldilocksExt2x8> for GoldilocksExt2x8 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: &'a Self) -> Self::Output {
        Self {
            c0: self.c0 - rhs.c0,
            c1: self.c1 - rhs.c1,
        }
    }
}

impl Sub for GoldilocksExt2x8 {
    type Output = Self;

    #[inline]
    #[allow(clippy::op_ref)]
    fn sub(self, rhs: Self) -> Self::Output {
        self - &rhs
    }
}

impl<'a> SubAssign<&'a GoldilocksExt2x8> for GoldilocksExt2x8 {
    #[inline]
    fn sub_assign(&mut self, rhs: &'a Self) {
        *self = *self - rhs;
    }
}

impl SubAssign for GoldilocksExt2x8 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}

impl Neg for GoldilocksExt2x8 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self {
            c0: -self.c0,
            c1: -self.c1,
        }
    }
}

impl<'a> Mul<&'a GoldilocksExt2x8> for GoldilocksExt2x8 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: &'a Self) -> Self::Output {
        // (a + bX)(c + dX) = ac + (ad + bc)X + bdX^2
        // where X^2 = 7
        // = (ac + 7bd) + (ad + bc)X
        let ac = self.c0 * rhs.c0;
        let bd = self.c1 * rhs.c1;
        let ad = self.c0 * rhs.c1;
        let bc = self.c1 * rhs.c0;
        Self {
            c0: ac + bd * Goldilocksx8::from(7u64),
            c1: ad + bc,
        }
    }
}

impl Mul for GoldilocksExt2x8 {
    type Output = Self;

    #[inline]
    #[allow(clippy::op_ref)]
    fn mul(self, rhs: Self) -> Self::Output {
        self * &rhs
    }
}

impl<'a> MulAssign<&'a GoldilocksExt2x8> for GoldilocksExt2x8 {
    #[inline]
    fn mul_assign(&mut self, rhs: &'a Self) {
        *self = *self * rhs;
    }
}

impl MulAssign for GoldilocksExt2x8 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self *= &rhs;
    }
}

impl<'a> Product<&'a GoldilocksExt2x8> for GoldilocksExt2x8 {
    fn product<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, x| acc * x)
    }
}

impl Product for GoldilocksExt2x8 {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, x| acc * x)
    }
}

impl Field for GoldilocksExt2x8 {
    const NAME: &'static str = "Goldilocks Extension Field 2x8";
    const SIZE: usize = 32;
    const FIELD_SIZE: usize = 0xFFFFFFFF00000001;
    const MODULUS: U256 = U256([0xFFFFFFFF00000001, 0]);

    const ZERO: Self = Self {
        c0: Goldilocksx8::ZERO,
        c1: Goldilocksx8::ZERO,
    };

    const ONE: Self = Self {
        c0: Goldilocksx8::ONE,
        c1: Goldilocksx8::ZERO,
    };

    const INV_2: Self = Self {
        c0: Goldilocksx8::INV_2,
        c1: Goldilocksx8::ZERO,
    };

    #[inline]
    fn zero() -> Self {
        Self::ZERO
    }

    #[inline]
    fn one() -> Self {
        Self::ONE
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.c0.is_zero() && self.c1.is_zero()
    }

    #[inline]
    fn random_unsafe(mut rng: impl RngCore) -> Self {
        Self {
            c0: Goldilocksx8::random_unsafe(&mut rng),
            c1: Goldilocksx8::random_unsafe(&mut rng),
        }
    }

    #[inline]
    fn random_bool(mut rng: impl RngCore) -> Self {
        Self {
            c0: Goldilocksx8::random_bool(&mut rng),
            c1: Goldilocksx8::ZERO,
        }
    }

    #[inline]
    fn as_u32_unchecked(&self) -> u32 {
        self.c0.as_u32_unchecked()
    }

    #[inline]
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        let mut c0_bytes = [0u8; 32];
        let mut c1_bytes = [0u8; 32];
        c0_bytes[..16].copy_from_slice(&bytes[..16]);
        c1_bytes[..16].copy_from_slice(&bytes[16..]);
        Self {
            c0: Goldilocksx8::from_uniform_bytes(&c0_bytes),
            c1: Goldilocksx8::from_uniform_bytes(&c1_bytes),
        }
    }

    #[inline]
    fn exp(&self, exponent: u128) -> Self {
        let mut base = *self;
        let mut result = Self::one();
        let mut exp = exponent;

        while exp != 0 {
            if exp & 1 == 1 {
                result *= &base;
            }
            base *= base;
            exp >>= 1;
        }
        result
    }

    #[inline]
    fn inv(&self) -> Option<Self> {
        if self.is_zero() {
            None
        } else {
            // For a + bX where X^2 = 7, the inverse is:
            // (a - bX) / (a^2 - 7b^2)
            let a2 = self.c0.square();
            let b2 = self.c1.square();
            let inv_norm = (a2 - b2 * Goldilocksx8::from(7u32)).inv()?;
            Some(Self {
                c0: self.c0 * inv_norm,
                c1: -self.c1 * inv_norm,
            })
        }
    }
}

impl ExtensionField for GoldilocksExt2x8 {
    type BaseField = Goldilocksx8;

    const DEGREE: usize = 2;
    const W: u32 = 7;
    const X: Self = Self {
        c0: Goldilocksx8::ZERO,
        c1: Goldilocksx8::ONE,
    };

    #[inline]
    fn mul_by_base_field(&self, base: &Self::BaseField) -> Self {
        Self {
            c0: self.c0 * base,
            c1: self.c1 * base,
        }
    }

    #[inline]
    fn add_by_base_field(&self, base: &Self::BaseField) -> Self {
        Self {
            c0: self.c0 + base,
            c1: self.c1,
        }
    }

    #[inline]
    fn mul_by_x(&self) -> Self {
        // (a + bX) * X = aX + bX^2
        // where X^2 = 7
        // = 7b + aX
        Self {
            c0: self.c1 * Goldilocksx8::from(7u32),
            c1: self.c0,
        }
    }

    #[inline]
    fn to_limbs(&self) -> Vec<Self::BaseField> {
        vec![self.c0, self.c1]
    }

    #[inline]
    fn from_limbs(limbs: &[Self::BaseField]) -> Self {
        assert!(limbs.len() >= 2);
        Self {
            c0: limbs[0],
            c1: limbs[1],
        }
    }
}

impl From<u32> for GoldilocksExt2x8 {
    #[inline]
    fn from(value: u32) -> Self {
        Self {
            c0: Goldilocksx8::from(value),
            c1: Goldilocksx8::ZERO,
        }
    }
}

impl From<Goldilocksx8> for GoldilocksExt2x8 {
    #[inline]
    fn from(x: Goldilocksx8) -> Self {
        Self {
            c0: x,
            c1: Goldilocksx8::ZERO,
        }
    }
}

impl Mul<Goldilocksx8> for GoldilocksExt2x8 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Goldilocksx8) -> Self {
        self.mul_by_base_field(&rhs)
    }
}

impl SimdField for GoldilocksExt2x8 {
    type Scalar = Self;

    const PACK_SIZE: usize = 1;

    #[inline]
    fn scale(&self, challenge: &Self::Scalar) -> Self {
        *self * challenge
    }

    #[inline]
    fn pack(base_vec: &[Self::Scalar]) -> Self {
        assert_eq!(base_vec.len(), Self::PACK_SIZE);
        base_vec[0]
    }

    #[inline]
    fn unpack(&self) -> Vec<Self::Scalar> {
        vec![*self]
    }
}

impl Hash for GoldilocksExt2x8 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.c0.hash(state);
        self.c1.hash(state);
    }
}
