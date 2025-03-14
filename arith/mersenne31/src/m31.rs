use std::{
    io::{Read, Write},
    iter::{Product, Sum},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::{field_common, Field, FieldForECC, FieldSerde, FieldSerdeResult};
use ark_std::Zero;
use rand::RngCore;

pub const M31_MOD: u32 = 2147483647;

#[inline]
pub(crate) fn mod_reduce_u32(x: u32) -> u32 {
    (x & M31_MOD) + (x >> 31)
}

#[inline]
fn mod_reduce_i64(x: i64) -> i64 {
    (x & M31_MOD as i64) + (x >> 31)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct M31 {
    pub v: u32,
}

field_common!(M31);

impl FieldSerde for M31 {
    const SERIALIZED_SIZE: usize = 32 / 8;

    #[inline(always)]
    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        writer.write_all(self.v.to_le_bytes().as_ref())?;
        Ok(())
    }

    // FIXME: this deserialization function auto corrects invalid inputs.
    // We should use separate APIs for this and for the actual deserialization.
    #[inline(always)]
    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let mut u = [0u8; Self::SERIALIZED_SIZE];
        reader.read_exact(&mut u)?;
        let mut v = u32::from_le_bytes(u);
        v = mod_reduce_u32(v);
        Ok(M31 { v })
    }
}

impl M31 {
    // Add two M31 without mod reduction
    #[inline(always)]
    pub fn unsafe_add(&self, rhs: &Self) -> Self {
        Self { v: self.v + rhs.v }
    }

    // Double an M31 without mod reduction
    #[inline(always)]
    pub fn unsafe_double(&self) -> Self {
        Self { v: self.v << 1 }
    }
}

impl Field for M31 {
    const NAME: &'static str = "Mersenne 31";

    const SIZE: usize = 32 / 8;

    const ZERO: Self = M31 { v: 0 };

    const ONE: Self = M31 { v: 1 };

    const INV_2: M31 = M31 { v: 1 << 30 };

    const FIELD_SIZE: usize = 32;

    #[inline(always)]
    fn zero() -> Self {
        M31 { v: 0 }
    }

    #[inline(always)]
    fn one() -> Self {
        M31 { v: 1 }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.v == 0 || self.v == M31_MOD
    }

    fn random_unsafe(mut rng: impl RngCore) -> Self {
        rng.next_u32().into()
    }

    fn random_bool(mut rng: impl RngCore) -> Self {
        (rng.next_u32() & 1).into()
    }

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

    fn inv(&self) -> Option<Self> {
        self.try_inverse()
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        self.v
    }

    #[inline(always)]
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        let mut v = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        v = mod_reduce_u32(v);
        M31 { v }
    }

    #[inline(always)]
    fn mul_by_5(&self) -> Self {
        *self * Self { v: 5 }
    }

    #[inline(always)]
    fn mul_by_6(&self) -> Self {
        *self * Self { v: 6 }
    }
}

impl FieldForECC for M31 {
    const MODULUS: ethnum::U256 = ethnum::U256::new(M31_MOD as u128);

    fn from_u256(x: ethnum::U256) -> Self {
        M31 {
            v: (x % ethnum::U256::from(M31_MOD)).as_u32(),
        }
    }
    fn to_u256(&self) -> ethnum::U256 {
        ethnum::U256::from(mod_reduce_u32(self.v))
    }
}

impl Neg for M31 {
    type Output = M31;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        M31 {
            v: if self.v == 0 { 0 } else { M31_MOD - self.v },
        }
    }
}

impl From<u32> for M31 {
    #[inline(always)]
    fn from(x: u32) -> Self {
        M31 {
            v: if x < M31_MOD { x } else { x % M31_MOD },
        }
    }
}

impl M31 {
    #[inline(always)]
    fn exp_power_of_2(&self, power_log: usize) -> Self {
        let mut res = *self;
        for _ in 0..power_log {
            res = res.square();
        }
        res
    }

    /// credit: https://github.com/Plonky3/Plonky3/blob/ed21a5e11cb20effadaab606598ccad4e70e1a3e/mersenne-31/src/mersenne_31.rs#L235
    #[inline(always)]
    fn try_inverse(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }

        // From Fermat's little theorem, in a prime field `F_p`, the inverse of `a` is `a^(p-2)`.
        // Here p-2 = 2147483646 = 1111111111111111111111111111101_2.
        // Uses 30 Squares + 7 Multiplications => 37 Operations total.

        let p1 = *self;
        let p101 = p1.exp_power_of_2(2) * p1;
        let p1111 = p101.square() * p101;
        let p11111111 = p1111.exp_power_of_2(4) * p1111;
        let p111111110000 = p11111111.exp_power_of_2(4);
        let p111111111111 = p111111110000 * p1111;
        let p1111111111111111 = p111111110000.exp_power_of_2(4) * p11111111;
        let p1111111111111111111111111111 = p1111111111111111.exp_power_of_2(12) * p111111111111;
        let p1111111111111111111111111111101 =
            p1111111111111111111111111111.exp_power_of_2(3) * p101;
        Some(p1111111111111111111111111111101)
    }
}

#[inline(always)]
fn add_internal(a: &M31, b: &M31) -> M31 {
    let mut vv = a.v + b.v;
    if vv >= M31_MOD {
        vv -= M31_MOD;
    }
    M31 { v: vv }
}

#[inline(always)]
fn sub_internal(a: &M31, b: &M31) -> M31 {
    let mut vv = a.v + M31_MOD - b.v;
    if vv >= M31_MOD {
        vv -= M31_MOD;
    }
    M31 { v: vv }
}

#[inline(always)]
fn mul_internal(a: &M31, b: &M31) -> M31 {
    let mut vv = a.v as i64 * b.v as i64;
    vv = mod_reduce_i64(vv);

    if vv >= M31_MOD as i64 {
        vv -= M31_MOD as i64;
    }
    M31 { v: vv as u32 }
}
