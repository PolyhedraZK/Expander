use std::{
    fmt::Debug,
    iter::{Product, Sum},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::{field_common, FFTField, Field, SimdField};
use ethnum::U256;
use rand::RngCore;
use serdes::ExpSerde;

use crate::BabyBear;

const BABY_BEAR_PACK_SIZE: usize = 16;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, ExpSerde)]
pub struct ScalarBabyBear {
    pub v: [BabyBear; 16],
}

field_common!(ScalarBabyBear);

impl Field for ScalarBabyBear {
    const NAME: &'static str = "Scalar Packed BabyBear";

    const SIZE: usize = 64;

    const FIELD_SIZE: usize = 32;

    const ZERO: Self = Self {
        v: [BabyBear::ZERO; 16],
    };

    const ONE: Self = Self {
        v: [BabyBear::ONE; 16],
    };

    const INV_2: Self = Self {
        v: [BabyBear::INV_2; 16],
    };

    const MODULUS: U256 = BabyBear::MODULUS;

    #[inline(always)]
    fn zero() -> Self {
        Self {
            v: [BabyBear::zero(); 16],
        }
    }

    #[inline(always)]
    fn one() -> Self {
        Self {
            v: [BabyBear::one(); 16],
        }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.v.iter().all(|x| x.is_zero())
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl RngCore) -> Self {
        let mut v = [BabyBear::zero(); 16];
        for elem in &mut v {
            *elem = BabyBear::random_unsafe(&mut rng);
        }
        Self { v }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl RngCore) -> Self {
        let mut v = [BabyBear::zero(); 16];
        for elem in &mut v {
            *elem = BabyBear::random_bool(&mut rng);
        }
        Self { v }
    }

    #[inline(always)]
    fn from_uniform_bytes(bytes: &[u8]) -> Self {
        let m = BabyBear::from_uniform_bytes(bytes);
        Self::pack_full(&m)
    }

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        if self.v.iter().any(|x| x.is_zero()) {
            return None;
        }

        let mut res = Self::zero();
        for i in 0..16 {
            res.v[i] = self.v[i].inv()?;
        }
        Some(res)
    }

    fn as_u32_unchecked(&self) -> u32 {
        unimplemented!("self is a vector, cannot convert to u32")
    }
}

impl SimdField for ScalarBabyBear {
    type Scalar = BabyBear;

    const PACK_SIZE: usize = BABY_BEAR_PACK_SIZE;

    #[inline(always)]
    fn pack_full(x: &BabyBear) -> Self {
        Self { v: [*x; 16] }
    }

    #[inline]
    fn scale(&self, challenge: &Self::Scalar) -> Self {
        let res = self.v.map(|x| x * challenge);
        Self { v: res }
    }

    #[inline(always)]
    fn pack(base_vec: &[Self::Scalar]) -> Self {
        assert!(base_vec.len() == BABY_BEAR_PACK_SIZE);
        let mut v = [BabyBear::zero(); 16];
        v.copy_from_slice(base_vec);
        Self { v }
    }

    #[inline(always)]
    fn unpack(&self) -> Vec<Self::Scalar> {
        self.v.to_vec()
    }
}

impl From<BabyBear> for ScalarBabyBear {
    #[inline(always)]
    fn from(x: BabyBear) -> Self {
        Self { v: [x; 16] }
    }
}

impl From<u32> for ScalarBabyBear {
    #[inline(always)]
    fn from(x: u32) -> Self {
        Self::pack_full(&BabyBear::new(x))
    }
}

impl Neg for ScalarBabyBear {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self {
        let mut res = Self::zero();
        for i in 0..16 {
            res.v[i] = -self.v[i];
        }
        res
    }
}

impl Mul<&BabyBear> for ScalarBabyBear {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: &BabyBear) -> Self::Output {
        let res = self.v.map(|x| x * rhs);
        Self { v: res }
    }
}

impl Mul<BabyBear> for ScalarBabyBear {
    type Output = Self;

    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn mul(self, rhs: BabyBear) -> Self::Output {
        self * &rhs
    }
}

impl Add<BabyBear> for ScalarBabyBear {
    type Output = ScalarBabyBear;
    #[inline(always)]
    fn add(self, rhs: BabyBear) -> Self::Output {
        self + ScalarBabyBear::pack_full(&rhs)
    }
}

#[inline(always)]
fn add_internal(a: &ScalarBabyBear, b: &ScalarBabyBear) -> ScalarBabyBear {
    let mut res = ScalarBabyBear::zero();
    for i in 0..16 {
        res.v[i] = a.v[i] + b.v[i];
    }
    res
}

#[inline(always)]
fn sub_internal(a: &ScalarBabyBear, b: &ScalarBabyBear) -> ScalarBabyBear {
    let mut res = ScalarBabyBear::zero();
    for i in 0..16 {
        res.v[i] = a.v[i] - b.v[i];
    }
    res
}

#[inline(always)]
fn mul_internal(a: &ScalarBabyBear, b: &ScalarBabyBear) -> ScalarBabyBear {
    let mut res = ScalarBabyBear::zero();
    for i in 0..16 {
        res.v[i] = a.v[i] * b.v[i];
    }
    res
}
