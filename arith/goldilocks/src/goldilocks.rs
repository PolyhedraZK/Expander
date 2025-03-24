use std::{
    io::{Read, Write},
    iter::{Product, Sum},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::{field_common, FFTField, Field};
use ark_std::Zero;
use ethnum::U256;
use rand::RngCore;
use serdes::{ExpSerde, SerdeResult};

use crate::util::{
    add_no_canonicalize_trashing_input, assume, branch_hint, split, try_inverse_u64,
};

// Goldilocks field modulus: 2^64 - 2^32 + 1
pub const GOLDILOCKS_MOD: u64 = 0xFFFFFFFF00000001;
/// 2^32 - 1
pub const EPSILON: u64 = 0xffffffff;

#[inline(always)]
pub(crate) fn mod_reduce_u64(x: u64) -> u64 {
    if x >= GOLDILOCKS_MOD {
        x - GOLDILOCKS_MOD
    } else {
        x
    }
}

#[derive(Debug, Clone, Copy, Default, PartialOrd, Ord)]
pub struct Goldilocks {
    pub v: u64,
}

impl PartialEq for Goldilocks {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        mod_reduce_u64(self.v) == mod_reduce_u64(other.v)
    }
}

impl Eq for Goldilocks {}

field_common!(Goldilocks);

impl ExpSerde for Goldilocks {
    const SERIALIZED_SIZE: usize = 64 / 8;

    #[inline(always)]
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        writer.write_all(self.v.to_le_bytes().as_ref())?;
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let mut u = [0u8; Self::SERIALIZED_SIZE];
        reader.read_exact(&mut u)?;
        let mut v = u64::from_le_bytes(u);
        v = mod_reduce_u64(v);
        Ok(Goldilocks { v })
    }
}

// impl Goldilocks {
//     #[inline(always)]
//     pub fn unsafe_add(&self, rhs: &Self) -> Self {
//         Self { v: self.v + rhs.v }
//     }

//     #[inline(always)]
//     pub fn unsafe_double(&self) -> Self {
//         Self { v: self.v << 1 }
//     }
// }

impl Field for Goldilocks {
    const NAME: &'static str = "Goldilocks";

    const SIZE: usize = 64 / 8;

    const ZERO: Self = Goldilocks { v: 0 };

    const ONE: Self = Goldilocks { v: 1 };

    const INV_2: Self = Goldilocks {
        v: 0x7FFFFFFF80000001,
    }; // (2^63 - 2^31 + 1)

    const FIELD_SIZE: usize = 64;

    const MODULUS: U256 = U256([GOLDILOCKS_MOD as u128, 0]);

    #[inline(always)]
    fn zero() -> Self {
        Goldilocks { v: 0 }
    }

    #[inline(always)]
    fn one() -> Self {
        Goldilocks { v: 1 }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.v == 0 || self.v == GOLDILOCKS_MOD
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl RngCore) -> Self {
        rng.next_u64().into()
    }

    #[inline(always)]
    fn random_bool(mut rng: impl RngCore) -> Self {
        (rng.next_u64() & 1).into()
    }

    #[inline(always)]
    fn to_u256(&self) -> U256 {
        U256([self.v as u128, 0])
    }

    #[inline(always)]
    fn from_u256(value: U256) -> Self {
        // TODO: this is a hack to get the low 64 bits of the u256
        // TODO: we should remove the assumption that the top bits are 0s
        let (_high, low) = value.into_words();
        let mut v = low as u64;
        v = mod_reduce_u64(v);
        Goldilocks { v }
    }

    #[inline]
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

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        self.try_inverse()
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        self.v as u32
    }

    #[inline(always)]
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        let mut v = u64::from_le_bytes(bytes[..8].try_into().unwrap());
        v = mod_reduce_u64(v);
        Goldilocks { v }
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

impl Neg for Goldilocks {
    type Output = Goldilocks;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        Goldilocks::ZERO - self
    }
}

impl From<u32> for Goldilocks {
    #[inline(always)]
    fn from(x: u32) -> Self {
        Goldilocks { v: x as u64 }
    }
}

impl From<u64> for Goldilocks {
    #[inline(always)]
    fn from(x: u64) -> Self {
        Goldilocks {
            v: if x < GOLDILOCKS_MOD {
                x
            } else {
                x - GOLDILOCKS_MOD
            },
        }
    }
}

impl Goldilocks {
    #[inline(always)]
    pub fn exp_power_of_2(&self, power_log: usize) -> Self {
        let mut res = *self;
        for _ in 0..power_log {
            res = res.square();
        }
        res
    }

    #[inline(always)]
    fn try_inverse(&self) -> Option<Self> {
        try_inverse_u64(&self.v).map(|v| Goldilocks { v })
    }

    #[inline(always)]
    pub fn mul_by_7(&self) -> Self {
        *self * Self { v: 7 }
    }
}

#[inline(always)]
/// credit: plonky2
fn add_internal(a: &Goldilocks, b: &Goldilocks) -> Goldilocks {
    let (sum, over) = a.v.overflowing_add(b.v);
    let (mut sum, over) = sum.overflowing_add((over as u64) * EPSILON);
    if over {
        // NB: self.0 > Self::ORDER && rhs.0 > Self::ORDER is necessary but not sufficient for
        // double-overflow.
        // This assume does two things:
        //  1. If compiler knows that either self.0 or rhs.0 <= ORDER, then it can skip this check.
        //  2. Hints to the compiler how rare this double-overflow is (thus handled better with a
        //     branch).
        assume(a.v > GOLDILOCKS_MOD && b.v > GOLDILOCKS_MOD);
        branch_hint();
        sum += EPSILON; // Cannot overflow.
    }
    Goldilocks { v: sum }
}

#[inline(always)]
fn sub_internal(a: &Goldilocks, b: &Goldilocks) -> Goldilocks {
    let (diff, under) = a.v.overflowing_sub(b.v);
    let (mut diff, under) = diff.overflowing_sub((under as u64) * EPSILON);
    if under {
        // NB: self.0 < EPSILON - 1 && rhs.0 > Self::ORDER is necessary but not sufficient for
        // double-underflow.
        // This assume does two things:
        //  1. If compiler knows that either self.0 >= EPSILON - 1 or rhs.0 <= ORDER, then it can
        //     skip this check.
        //  2. Hints to the compiler how rare this double-underflow is (thus handled better with a
        //     branch).
        assume(a.v < EPSILON - 1 && b.v > GOLDILOCKS_MOD);
        branch_hint();
        diff -= EPSILON; // Cannot underflow.
    }
    Goldilocks { v: diff }
}

#[inline(always)]
fn mul_internal(a: &Goldilocks, b: &Goldilocks) -> Goldilocks {
    reduce128((a.v as u128) * (b.v as u128))
}

/// Reduces to a 64-bit value. The result might not be in canonical form; it could be in between the
/// field order and `2^64`.
#[inline]
fn reduce128(x: u128) -> Goldilocks {
    let (x_lo, x_hi) = split(x); // This is a no-op
    let x_hi_hi = x_hi >> 32;
    let x_hi_lo = x_hi & EPSILON;

    let (mut t0, borrow) = x_lo.overflowing_sub(x_hi_hi);
    if borrow {
        branch_hint(); // A borrow is exceedingly rare. It is faster to branch.
        t0 -= EPSILON; // Cannot underflow.
    }
    let t1 = x_hi_lo * EPSILON;
    let t2 = unsafe { add_no_canonicalize_trashing_input(t0, t1) };
    Goldilocks { v: t2 }
}

impl std::hash::Hash for Goldilocks {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.v);
    }
}

impl FFTField for Goldilocks {
    const TWO_ADICITY: usize = 32; // 2^32 divides p-1

    /// The `2^s` root of unity.
    ///
    /// It can be calculated by exponentiating `Self::MULTIPLICATIVE_GENERATOR` by `t`,
    /// where `t = (modulus - 1) >> Self::S`.
    #[inline(always)]
    fn root_of_unity() -> Self {
        Goldilocks {
            v: 0x185629dcda58878c,
        } // 5 is a primitive root of order 2^32
    }
}
