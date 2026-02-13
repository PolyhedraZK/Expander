use std::{
    hash::Hasher,
    mem::transmute,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::Field;
use ethnum::U256;
use serdes::{ExpSerde, SerdeResult};

use crate::GF2;

#[derive(Clone, Copy, Debug)]
pub struct ScalarGF2x128 {
    pub(crate) v: [u64; 2],
}

impl ExpSerde for ScalarGF2x128 {
    #[inline(always)]
    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        unsafe { writer.write_all(transmute::<[u64; 2], [u8; 16]>(self.v).as_ref())? };
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: std::io::Read>(mut reader: R) -> SerdeResult<Self> {
        let mut u = [0u8; 16];
        reader.read_exact(&mut u)?;
        Ok(ScalarGF2x128 {
            v: unsafe { transmute::<[u8; 16], [u64; 2]>(u) },
        })
    }
}

impl Field for ScalarGF2x128 {
    const NAME: &'static str = "Scalar Galois Field 2 SIMD 128";

    const SIZE: usize = 128 / 8;

    const FIELD_SIZE: usize = 1;

    const ZERO: Self = ScalarGF2x128 { v: [0u64; 2] };

    const ONE: Self = ScalarGF2x128 { v: [!0u64; 2] };

    const INV_2: Self = ScalarGF2x128 { v: [0u64; 2] };

    const MODULUS: U256 = unimplemented!();

    #[inline(always)]
    fn zero() -> Self {
        ScalarGF2x128 { v: [0u64; 2] }
    }

    #[inline(always)]
    fn one() -> Self {
        ScalarGF2x128 { v: [!0u64; 2] }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.v[0] == 0 && self.v[1] == 0
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        let mut u = [0u8; 16];
        rng.fill_bytes(&mut u);
        ScalarGF2x128 {
            v: unsafe { transmute::<[u8; 16], [u64; 2]>(u) },
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        let mut u = [0u8; 16];
        rng.fill_bytes(&mut u);
        ScalarGF2x128 {
            v: unsafe { transmute::<[u8; 16], [u64; 2]>(u) },
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
        unimplemented!("u32 for GF2x128 doesn't make sense")
    }

    #[inline(always)]
    fn from_uniform_bytes(bytes: &[u8]) -> Self {
        let arr: [u8; 16] = bytes[..16].try_into().unwrap();
        ScalarGF2x128 {
            v: unsafe { transmute::<[u8; 16], [u64; 2]>(arr) },
        }
    }
}

impl Default for ScalarGF2x128 {
    #[inline(always)]
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialEq for ScalarGF2x128 {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.v == other.v
    }
}

impl Mul<&ScalarGF2x128> for ScalarGF2x128 {
    type Output = ScalarGF2x128;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: &ScalarGF2x128) -> ScalarGF2x128 {
        ScalarGF2x128 {
            v: [self.v[0] & rhs.v[0], self.v[1] & rhs.v[1]],
        }
    }
}

impl Mul<ScalarGF2x128> for ScalarGF2x128 {
    type Output = ScalarGF2x128;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: ScalarGF2x128) -> ScalarGF2x128 {
        ScalarGF2x128 {
            v: [self.v[0] & rhs.v[0], self.v[1] & rhs.v[1]],
        }
    }
}

impl MulAssign<&ScalarGF2x128> for ScalarGF2x128 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn mul_assign(&mut self, rhs: &ScalarGF2x128) {
        self.v[0] &= rhs.v[0];
        self.v[1] &= rhs.v[1];
    }
}

impl MulAssign<ScalarGF2x128> for ScalarGF2x128 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn mul_assign(&mut self, rhs: ScalarGF2x128) {
        self.v[0] &= rhs.v[0];
        self.v[1] &= rhs.v[1];
    }
}

impl Sub for ScalarGF2x128 {
    type Output = ScalarGF2x128;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: ScalarGF2x128) -> ScalarGF2x128 {
        ScalarGF2x128 {
            v: [self.v[0] ^ rhs.v[0], self.v[1] ^ rhs.v[1]],
        }
    }
}

impl SubAssign for ScalarGF2x128 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn sub_assign(&mut self, rhs: ScalarGF2x128) {
        self.v[0] ^= rhs.v[0];
        self.v[1] ^= rhs.v[1];
    }
}

impl Add for ScalarGF2x128 {
    type Output = ScalarGF2x128;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: ScalarGF2x128) -> ScalarGF2x128 {
        ScalarGF2x128 {
            v: [self.v[0] ^ rhs.v[0], self.v[1] ^ rhs.v[1]],
        }
    }
}

impl AddAssign for ScalarGF2x128 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: ScalarGF2x128) {
        self.v[0] ^= rhs.v[0];
        self.v[1] ^= rhs.v[1];
    }
}

impl Add<&ScalarGF2x128> for ScalarGF2x128 {
    type Output = ScalarGF2x128;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: &ScalarGF2x128) -> ScalarGF2x128 {
        ScalarGF2x128 {
            v: [self.v[0] ^ rhs.v[0], self.v[1] ^ rhs.v[1]],
        }
    }
}

impl AddAssign<&ScalarGF2x128> for ScalarGF2x128 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: &ScalarGF2x128) {
        self.v[0] ^= rhs.v[0];
        self.v[1] ^= rhs.v[1];
    }
}

impl Sub<&ScalarGF2x128> for ScalarGF2x128 {
    type Output = ScalarGF2x128;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: &ScalarGF2x128) -> ScalarGF2x128 {
        ScalarGF2x128 {
            v: [self.v[0] ^ rhs.v[0], self.v[1] ^ rhs.v[1]],
        }
    }
}

impl SubAssign<&ScalarGF2x128> for ScalarGF2x128 {
    #[inline(always)]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn sub_assign(&mut self, rhs: &ScalarGF2x128) {
        self.v[0] ^= rhs.v[0];
        self.v[1] ^= rhs.v[1];
    }
}

impl<T: std::borrow::Borrow<ScalarGF2x128>> std::iter::Sum<T> for ScalarGF2x128 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl<T: std::borrow::Borrow<ScalarGF2x128>> std::iter::Product<T> for ScalarGF2x128 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Neg for ScalarGF2x128 {
    type Output = ScalarGF2x128;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn neg(self) -> ScalarGF2x128 {
        ScalarGF2x128 { v: self.v }
    }
}

impl From<u32> for ScalarGF2x128 {
    #[inline(always)]
    fn from(v: u32) -> Self {
        assert!(v < 2);
        if v == 0 {
            ScalarGF2x128::ZERO
        } else {
            ScalarGF2x128::ONE
        }
    }
}

impl From<GF2> for ScalarGF2x128 {
    #[inline(always)]
    fn from(v: GF2) -> Self {
        assert!(v.v < 2);
        if v.v == 0 {
            ScalarGF2x128::ZERO
        } else {
            ScalarGF2x128::ONE
        }
    }
}

impl std::hash::Hash for ScalarGF2x128 {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe {
            state.write(transmute::<[u64; 2], [u8; 16]>(self.v).as_ref());
        }
    }
}
