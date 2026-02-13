use std::{
    fmt::Debug,
    iter::{Product, Sum},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::{field_common, Field, SimdField};
use ethnum::U256;
use rand::RngCore;
use serdes::ExpSerde;

use crate::{m31::M31_MOD, M31};

const M31_PACK_SIZE: usize = 16;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, ExpSerde)]
pub struct ScalarM31x16 {
    pub v: [M31; 16],
}

field_common!(ScalarM31x16);

impl Field for ScalarM31x16 {
    const NAME: &'static str = "Scalar Packed Mersenne 31";

    const SIZE: usize = 64;

    const FIELD_SIZE: usize = 32;

    const ZERO: Self = Self {
        v: [M31::ZERO; 16],
    };

    const ONE: Self = Self {
        v: [M31 { v: 1 }; 16],
    };

    const INV_2: Self = Self {
        v: [M31 { v: 1 << 30 }; 16],
    };

    const MODULUS: U256 = M31::MODULUS;

    #[inline(always)]
    fn zero() -> Self {
        Self {
            v: [M31::zero(); 16],
        }
    }

    #[inline(always)]
    fn one() -> Self {
        Self {
            v: [M31::one(); 16],
        }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.v.iter().all(|x| x.is_zero())
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl RngCore) -> Self {
        let mut v = [M31::zero(); 16];
        for elem in &mut v {
            *elem = M31::random_unsafe(&mut rng);
        }
        Self { v }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl RngCore) -> Self {
        let mut v = [M31::zero(); 16];
        for elem in &mut v {
            *elem = M31::random_bool(&mut rng);
        }
        Self { v }
    }

    #[inline(always)]
    fn from_uniform_bytes(bytes: &[u8]) -> Self {
        let m = M31::from_uniform_bytes(bytes);
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

impl SimdField for ScalarM31x16 {
    type Scalar = M31;

    const PACK_SIZE: usize = M31_PACK_SIZE;

    #[inline(always)]
    fn pack_full(x: &M31) -> Self {
        Self { v: [*x; 16] }
    }

    #[inline]
    fn scale(&self, challenge: &Self::Scalar) -> Self {
        let res = self.v.map(|x| x * challenge);
        Self { v: res }
    }

    #[inline(always)]
    fn pack(base_vec: &[Self::Scalar]) -> Self {
        assert!(base_vec.len() == M31_PACK_SIZE);
        let mut v = [M31::zero(); 16];
        v.copy_from_slice(base_vec);
        Self { v }
    }

    #[inline(always)]
    fn unpack(&self) -> Vec<Self::Scalar> {
        self.v.to_vec()
    }

    #[inline(always)]
    fn horizontal_sum(&self) -> Self::Scalar {
        let mut buffer: u64 = self.v[0].v as u64;
        buffer += self.v[1].v as u64;
        buffer += self.v[2].v as u64;
        buffer += self.v[3].v as u64;
        buffer += self.v[4].v as u64;
        buffer += self.v[5].v as u64;
        buffer += self.v[6].v as u64;
        buffer += self.v[7].v as u64;
        buffer += self.v[8].v as u64;
        buffer += self.v[9].v as u64;
        buffer += self.v[10].v as u64;
        buffer += self.v[11].v as u64;
        buffer += self.v[12].v as u64;
        buffer += self.v[13].v as u64;
        buffer += self.v[14].v as u64;
        buffer += self.v[15].v as u64;

        buffer = (buffer & M31_MOD as u64) + (buffer >> 31);
        if buffer == M31_MOD as u64 {
            Self::Scalar::ZERO
        } else {
            Self::Scalar { v: buffer as u32 }
        }
    }
}

impl From<M31> for ScalarM31x16 {
    #[inline(always)]
    fn from(x: M31) -> Self {
        ScalarM31x16::pack_full(&x)
    }
}

impl From<u32> for ScalarM31x16 {
    #[inline(always)]
    fn from(x: u32) -> Self {
        ScalarM31x16::pack_full(&M31::from(x))
    }
}

impl Neg for ScalarM31x16 {
    type Output = ScalarM31x16;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        ScalarM31x16::zero() - self
    }
}

impl Mul<&M31> for ScalarM31x16 {
    type Output = ScalarM31x16;
    #[inline(always)]
    fn mul(self, rhs: &M31) -> Self::Output {
        let rhs_p = ScalarM31x16::pack_full(rhs);
        self * rhs_p
    }
}

impl Mul<M31> for ScalarM31x16 {
    type Output = ScalarM31x16;
    #[inline(always)]
    fn mul(self, rhs: M31) -> Self::Output {
        self * &rhs
    }
}

impl Add<M31> for ScalarM31x16 {
    type Output = ScalarM31x16;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn add(self, rhs: M31) -> Self::Output {
        self + ScalarM31x16::pack_full(&rhs)
    }
}

#[inline(always)]
fn add_internal(a: &ScalarM31x16, b: &ScalarM31x16) -> ScalarM31x16 {
    let mut res = ScalarM31x16::zero();
    for i in 0..16 {
        res.v[i] = a.v[i] + b.v[i];
    }
    res
}

#[inline(always)]
fn sub_internal(a: &ScalarM31x16, b: &ScalarM31x16) -> ScalarM31x16 {
    let mut res = ScalarM31x16::zero();
    for i in 0..16 {
        res.v[i] = a.v[i] - b.v[i];
    }
    res
}

#[inline(always)]
fn mul_internal(a: &ScalarM31x16, b: &ScalarM31x16) -> ScalarM31x16 {
    let mut res = ScalarM31x16::zero();
    for i in 0..16 {
        res.v[i] = a.v[i] * b.v[i];
    }
    res
}
