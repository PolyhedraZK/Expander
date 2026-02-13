use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use arith::{field_common, ExtensionField, Field, SimdField};
use ethnum::U256;
use gf2::{GF2x8, GF2};
use serdes::{ExpSerde, SerdeResult};

use crate::gf2_ext128::scalar::ScalarGF2_128;
use crate::GF2_128;

#[derive(Clone, Copy, Debug)]
pub struct ScalarGF2_128x8 {
    pub v: [ScalarGF2_128; 8],
}

field_common!(ScalarGF2_128x8);

impl Default for ScalarGF2_128x8 {
    fn default() -> Self {
        Self::zero()
    }
}

impl PartialEq for ScalarGF2_128x8 {
    fn eq(&self, other: &Self) -> bool {
        self.v.iter().zip(other.v.iter()).all(|(a, b)| a == b)
    }
}

impl Eq for ScalarGF2_128x8 {}

impl ExpSerde for ScalarGF2_128x8 {
    #[inline(always)]
    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        for elem in &self.v {
            elem.serialize_into(&mut writer)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: std::io::Read>(mut reader: R) -> SerdeResult<Self> {
        let mut v = [ScalarGF2_128::ZERO; 8];
        for elem in v.iter_mut() {
            *elem = ScalarGF2_128::deserialize_from(&mut reader)?;
        }
        Ok(Self { v })
    }
}

impl Field for ScalarGF2_128x8 {
    const NAME: &'static str = "Scalar GF(2^128) SIMD 8";

    const SIZE: usize = 16 * 8;

    const FIELD_SIZE: usize = 128;

    const ZERO: Self = ScalarGF2_128x8 {
        v: [ScalarGF2_128 { v: [0, 0] }; 8],
    };

    const ONE: Self = ScalarGF2_128x8 {
        v: [ScalarGF2_128 { v: [1, 0] }; 8],
    };

    const INV_2: Self = ScalarGF2_128x8 {
        v: [ScalarGF2_128 { v: [0, 0] }; 8],
    };

    const MODULUS: U256 = unimplemented!();

    #[inline(always)]
    fn zero() -> Self {
        ScalarGF2_128x8 {
            v: [ScalarGF2_128::zero(); 8],
        }
    }

    #[inline(always)]
    fn one() -> Self {
        ScalarGF2_128x8 {
            v: [ScalarGF2_128::one(); 8],
        }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.v.iter().all(|e| e.is_zero())
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        ScalarGF2_128x8 {
            v: std::array::from_fn(|_| ScalarGF2_128::random_unsafe(&mut rng)),
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        ScalarGF2_128x8 {
            v: std::array::from_fn(|_| ScalarGF2_128::random_bool(&mut rng)),
        }
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
    fn square(&self) -> Self {
        self * self
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        unimplemented!("u32 for GF128 doesn't make sense")
    }

    #[inline(always)]
    fn from_uniform_bytes(_bytes: &[u8]) -> Self {
        unimplemented!("from_uniform_bytes for GF128 doesn't make sense")
    }
}

impl SimdField for ScalarGF2_128x8 {
    type Scalar = GF2_128;

    const PACK_SIZE: usize = 8;

    #[inline(always)]
    fn scale(&self, challenge: &Self::Scalar) -> Self {
        ScalarGF2_128x8 {
            v: std::array::from_fn(|i| self.v[i] * *challenge),
        }
    }

    #[inline]
    fn pack_full(base: &Self::Scalar) -> Self {
        Self { v: [*base; 8] }
    }

    #[inline(always)]
    fn pack(base_vec: &[Self::Scalar]) -> Self {
        assert!(base_vec.len() == 8);
        let base_vec_array: [Self::Scalar; 8] = base_vec.try_into().unwrap();
        Self { v: base_vec_array }
    }

    #[inline(always)]
    fn unpack(&self) -> Vec<Self::Scalar> {
        self.v.to_vec()
    }
}

impl From<ScalarGF2_128> for ScalarGF2_128x8 {
    fn from(v: ScalarGF2_128) -> Self {
        ScalarGF2_128x8 { v: [v; 8] }
    }
}

impl Neg for ScalarGF2_128x8 {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        self
    }
}

impl From<u32> for ScalarGF2_128x8 {
    fn from(v: u32) -> Self {
        ScalarGF2_128x8 {
            v: [ScalarGF2_128::from(v); 8],
        }
    }
}

#[inline(always)]
fn add_internal(a: &ScalarGF2_128x8, b: &ScalarGF2_128x8) -> ScalarGF2_128x8 {
    ScalarGF2_128x8 {
        v: std::array::from_fn(|i| a.v[i] + b.v[i]),
    }
}

#[inline(always)]
fn sub_internal(a: &ScalarGF2_128x8, b: &ScalarGF2_128x8) -> ScalarGF2_128x8 {
    add_internal(a, b)
}

#[inline(always)]
fn mul_internal(a: &ScalarGF2_128x8, b: &ScalarGF2_128x8) -> ScalarGF2_128x8 {
    ScalarGF2_128x8 {
        v: std::array::from_fn(|i| a.v[i] * b.v[i]),
    }
}

impl ExtensionField for ScalarGF2_128x8 {
    const DEGREE: usize = ScalarGF2_128::DEGREE;
    const W: u32 = ScalarGF2_128::W;
    const X: Self = Self {
        v: [ScalarGF2_128 { v: [2, 0] }; 8],
    };
    type BaseField = GF2x8;

    #[inline(always)]
    fn mul_by_base_field(&self, base: &Self::BaseField) -> Self {
        ScalarGF2_128x8 {
            v: std::array::from_fn(|i| {
                if (base.v >> i) & 1 == 1 {
                    self.v[i]
                } else {
                    ScalarGF2_128::ZERO
                }
            }),
        }
    }

    #[inline(always)]
    fn add_by_base_field(&self, base: &Self::BaseField) -> Self {
        ScalarGF2_128x8 {
            v: std::array::from_fn(|i| {
                let bit = ((base.v >> i) & 1) as u64;
                ScalarGF2_128 {
                    v: [self.v[i].v[0] ^ bit, self.v[i].v[1]],
                }
            }),
        }
    }

    #[inline(always)]
    fn mul_by_x(&self) -> Self {
        use crate::gf2_ext128::scalar::mul_by_x_scalar;
        ScalarGF2_128x8 {
            v: std::array::from_fn(|i| ScalarGF2_128 {
                v: mul_by_x_scalar(&self.v[i].v),
            }),
        }
    }

    #[inline(always)]
    fn from_limbs(limbs: &[Self::BaseField]) -> Self {
        let mut local_limbs = limbs.to_vec();
        local_limbs.resize(Self::DEGREE, Self::BaseField::ZERO);

        let mut buffer = vec![GF2::ZERO; Self::DEGREE * Self::PACK_SIZE];

        local_limbs.iter().enumerate().for_each(|(ith_limb, limb)| {
            let unpacked = limb.unpack();
            unpacked.iter().enumerate().for_each(|(ith_gf2, gf2_val)| {
                buffer[ith_gf2 * Self::DEGREE + ith_limb] = *gf2_val;
            });
        });

        let gf2_128s: Vec<_> = buffer
            .chunks(Self::DEGREE)
            .map(GF2_128::from_limbs)
            .collect();

        Self::pack(&gf2_128s)
    }

    #[inline(always)]
    fn to_limbs(&self) -> Vec<Self::BaseField> {
        let gf2_128s = self.unpack();

        let mut buffer = vec![GF2::ZERO; Self::DEGREE * Self::PACK_SIZE];
        gf2_128s
            .iter()
            .enumerate()
            .for_each(|(ith_gf2_128, gf2_128_val)| {
                let limbs = gf2_128_val.to_limbs();
                limbs.iter().enumerate().for_each(|(ith_limb, limb)| {
                    buffer[ith_limb * Self::PACK_SIZE + ith_gf2_128] = *limb;
                })
            });

        buffer.chunks(Self::PACK_SIZE).map(GF2x8::pack).collect()
    }
}

impl From<GF2x8> for ScalarGF2_128x8 {
    #[inline(always)]
    fn from(v: GF2x8) -> Self {
        ScalarGF2_128x8 {
            v: std::array::from_fn(|i| {
                let bit = ((v.v >> i) & 1) as u64;
                ScalarGF2_128 { v: [bit, 0] }
            }),
        }
    }
}

impl Mul<GF2x8> for ScalarGF2_128x8 {
    type Output = ScalarGF2_128x8;

    #[inline]
    fn mul(self, rhs: GF2x8) -> Self::Output {
        self.mul_by_base_field(&rhs)
    }
}

impl Mul<GF2> for ScalarGF2_128x8 {
    type Output = ScalarGF2_128x8;

    #[inline(always)]
    fn mul(self, rhs: GF2) -> Self::Output {
        if rhs.is_zero() {
            Self::zero()
        } else {
            self
        }
    }
}

impl Add<GF2> for ScalarGF2_128x8 {
    type Output = ScalarGF2_128x8;
    #[inline(always)]
    fn add(self, rhs: GF2) -> Self::Output {
        let bit = rhs.v as u64;
        ScalarGF2_128x8 {
            v: std::array::from_fn(|i| ScalarGF2_128 {
                v: [self.v[i].v[0] ^ bit, self.v[i].v[1]],
            }),
        }
    }
}

impl std::hash::Hash for ScalarGF2_128x8 {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for elem in &self.v {
            state.write(&elem.v[0].to_le_bytes());
            state.write(&elem.v[1].to_le_bytes());
        }
    }
}
