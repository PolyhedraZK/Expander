use std::{
    arch::aarch64::*,
    fmt::Debug,
    hash::Hash,
    io::{Read, Write},
    iter::{Product, Sum},
    mem::transmute,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::{field_common, Field, SimdField};
use ark_std::Zero;
use ethnum::U256;
use rand::{Rng, RngCore};
use serdes::{ExpSerde, SerdeResult};

use crate::{Goldilocks, EPSILON, GOLDILOCKS_MOD};

const GOLDILOCKS_PACK_SIZE: usize = 8;
const PACKED_MOD: uint64x2_t = unsafe { transmute([GOLDILOCKS_MOD; 2]) };
const PACKED_0: uint64x2_t = unsafe { transmute([0u64; 2]) };
const PACKED_INV_2: uint64x2_t = unsafe { transmute([0x7FFFFFFF80000001u64; 2]) };
const PACKED_EPSILON: uint64x2_t = unsafe { transmute([EPSILON; 2]) };

/// NeonGoldilocks packs 8 Goldilocks elements and operates on them in parallel
#[derive(Clone, Copy)]
pub struct NeonGoldilocks {
    pub v: [uint64x2_t; 4],
}

field_common!(NeonGoldilocks);

impl NeonGoldilocks {
    #[inline(always)]
    pub fn pack_full(x: Goldilocks) -> NeonGoldilocks {
        NeonGoldilocks {
            v: unsafe {
                [
                    vdupq_n_u64(x.v),
                    vdupq_n_u64(x.v),
                    vdupq_n_u64(x.v),
                    vdupq_n_u64(x.v),
                ]
            },
        }
    }
}

impl ExpSerde for NeonGoldilocks {
    const SERIALIZED_SIZE: usize = (128 / 8) * 4;

    #[inline(always)]
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        let data = unsafe { transmute::<[uint64x2_t; 4], [u8; 64]>(self.v) };
        writer.write_all(&data)?;
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let mut data = [0; 64];
        reader.read_exact(&mut data)?;
        unsafe {
            Ok(NeonGoldilocks {
                v: transmute::<[u8; 64], [uint64x2_t; 4]>(data),
            })
        }
    }
}

impl Field for NeonGoldilocks {
    const NAME: &'static str = "Neon Packed Goldilocks";
    const SIZE: usize = 128 / 8 * 4;
    const FIELD_SIZE: usize = 64;
    const ZERO: Self = Self { v: [PACKED_0; 4] };
    const ONE: Self = Self {
        v: [unsafe { transmute::<[u64; 2], uint64x2_t>([1; 2]) }; 4],
    };
    const INV_2: Self = Self {
        v: [PACKED_INV_2; 4],
    };
    const MODULUS: U256 = U256([GOLDILOCKS_MOD as u128, 0]);

    #[inline(always)]
    fn zero() -> Self {
        Self { v: [PACKED_0; 4] }
    }

    #[inline(always)]
    fn one() -> Self {
        Self::ONE
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        unsafe {
            transmute::<[uint64x2_t; 4], [u64; 8]>(self.v)
                .iter()
                .all(|&x| x == 0)
        }
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl RngCore) -> Self {
        unsafe {
            NeonGoldilocks {
                v: [
                    vld1q_u64(
                        [
                            rng.gen::<u64>() % GOLDILOCKS_MOD,
                            rng.gen::<u64>() % GOLDILOCKS_MOD,
                        ]
                        .as_ptr(),
                    ),
                    vld1q_u64(
                        [
                            rng.gen::<u64>() % GOLDILOCKS_MOD,
                            rng.gen::<u64>() % GOLDILOCKS_MOD,
                        ]
                        .as_ptr(),
                    ),
                    vld1q_u64(
                        [
                            rng.gen::<u64>() % GOLDILOCKS_MOD,
                            rng.gen::<u64>() % GOLDILOCKS_MOD,
                        ]
                        .as_ptr(),
                    ),
                    vld1q_u64(
                        [
                            rng.gen::<u64>() % GOLDILOCKS_MOD,
                            rng.gen::<u64>() % GOLDILOCKS_MOD,
                        ]
                        .as_ptr(),
                    ),
                ],
            }
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl RngCore) -> Self {
        unsafe {
            NeonGoldilocks {
                v: [
                    vld1q_u64([rng.gen::<bool>() as u64, rng.gen::<bool>() as u64].as_ptr()),
                    vld1q_u64([rng.gen::<bool>() as u64, rng.gen::<bool>() as u64].as_ptr()),
                    vld1q_u64([rng.gen::<bool>() as u64, rng.gen::<bool>() as u64].as_ptr()),
                    vld1q_u64([rng.gen::<bool>() as u64, rng.gen::<bool>() as u64].as_ptr()),
                ],
            }
        }
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
        // slow, should not be used in production
        let mut goldilocks_vec = unsafe { transmute::<[uint64x2_t; 4], [Goldilocks; 8]>(self.v) };
        let is_non_zero = goldilocks_vec.iter().all(|x| !x.is_zero());
        if !is_non_zero {
            return None;
        }

        goldilocks_vec
            .iter_mut()
            .for_each(|x| *x = x.inv().unwrap()); // safe unwrap
        Some(Self {
            v: unsafe { transmute::<[Goldilocks; 8], [uint64x2_t; 4]>(goldilocks_vec) },
        })
    }

    fn as_u32_unchecked(&self) -> u32 {
        unimplemented!("self is a vector, cannot convert to u32")
    }

    #[inline]
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        let m = Goldilocks::from_uniform_bytes(bytes);
        Self::pack_full(m)
    }

    #[inline(always)]
    fn mul_by_5(&self) -> Self {
        *self * Self::from(5u32)
    }

    #[inline(always)]
    fn mul_by_6(&self) -> Self {
        *self * Self::from(6u32)
    }
}

impl SimdField for NeonGoldilocks {
    type Scalar = Goldilocks;

    #[inline]
    fn scale(&self, challenge: &Self::Scalar) -> Self {
        let packed_challenge = NeonGoldilocks::pack_full(*challenge);
        *self * packed_challenge
    }

    const PACK_SIZE: usize = GOLDILOCKS_PACK_SIZE;

    #[inline(always)]
    fn pack(base_vec: &[Self::Scalar]) -> Self {
        assert!(base_vec.len() == GOLDILOCKS_PACK_SIZE);
        let ret: [Self::Scalar; GOLDILOCKS_PACK_SIZE] = base_vec.try_into().unwrap();
        unsafe { transmute(ret) }
    }

    #[inline(always)]
    fn unpack(&self) -> Vec<Self::Scalar> {
        let ret =
            unsafe { transmute::<[uint64x2_t; 4], [Self::Scalar; GOLDILOCKS_PACK_SIZE]>(self.v) };
        ret.to_vec()
    }
}

impl From<Goldilocks> for NeonGoldilocks {
    #[inline(always)]
    fn from(x: Goldilocks) -> Self {
        NeonGoldilocks::pack_full(x)
    }
}

impl From<u64> for NeonGoldilocks {
    #[inline(always)]
    fn from(x: u64) -> Self {
        NeonGoldilocks::pack_full(Goldilocks::from(x))
    }
}

impl Debug for NeonGoldilocks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for &v in self.v.iter() {
            unsafe {
                let data = [vgetq_lane_u64(v, 0), vgetq_lane_u64(v, 1)];
                // if all data is the same, print only one
                if data.iter().all(|&x| x == data[0]) {
                    write!(
                        f,
                        "uint64x2_t<8 x {}>",
                        if GOLDILOCKS_MOD - data[0] > 1024 {
                            format!("{}", data[0])
                        } else {
                            format!("-{}", GOLDILOCKS_MOD - data[0])
                        }
                    )?;
                } else {
                    write!(f, "uint64x2_t<{:?}>", data)?;
                }
            }
        }
        Ok(())
    }
}

impl Default for NeonGoldilocks {
    fn default() -> Self {
        NeonGoldilocks::zero()
    }
}

impl PartialEq for NeonGoldilocks {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            transmute::<[uint64x2_t; 4], [u64; 8]>(self.v)
                == transmute::<[uint64x2_t; 4], [u64; 8]>(other.v)
        }
    }
}

impl Eq for NeonGoldilocks {}

impl Mul<&Goldilocks> for NeonGoldilocks {
    type Output = NeonGoldilocks;
    #[inline(always)]
    fn mul(self, rhs: &Goldilocks) -> Self::Output {
        let rhs_p = NeonGoldilocks::pack_full(*rhs);
        self * rhs_p
    }
}

impl Mul<Goldilocks> for NeonGoldilocks {
    type Output = NeonGoldilocks;
    #[inline(always)]
    fn mul(self, rhs: Goldilocks) -> Self::Output {
        self * &rhs
    }
}

impl Add<Goldilocks> for NeonGoldilocks {
    type Output = NeonGoldilocks;
    #[inline(always)]
    fn add(self, rhs: Goldilocks) -> Self::Output {
        self + NeonGoldilocks::pack_full(rhs)
    }
}

impl From<u32> for NeonGoldilocks {
    #[inline(always)]
    fn from(x: u32) -> Self {
        NeonGoldilocks::pack_full(Goldilocks::from(x))
    }
}

impl Neg for NeonGoldilocks {
    type Output = NeonGoldilocks;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        NeonGoldilocks::zero() - self
    }
}

#[inline]
fn add_internal(a: &NeonGoldilocks, b: &NeonGoldilocks) -> NeonGoldilocks {
    let mut res = NeonGoldilocks::zero();
    for i in 0..4 {
        unsafe {
            // Extract values
            let a_vals = [vgetq_lane_u64(a.v[i], 0), vgetq_lane_u64(a.v[i], 1)];
            let b_vals = [vgetq_lane_u64(b.v[i], 0), vgetq_lane_u64(b.v[i], 1)];

            // Perform addition and modular reduction
            let mut res_vals = [0u64; 2];
            for j in 0..2 {
                let sum = a_vals[j].wrapping_add(b_vals[j]);
                res_vals[j] = if sum >= GOLDILOCKS_MOD {
                    sum - GOLDILOCKS_MOD
                } else {
                    sum
                };
            }

            // Pack results back
            res.v[i] = vld1q_u64(res_vals.as_ptr());
        }
    }
    res
}

#[inline]
fn sub_internal(a: &NeonGoldilocks, b: &NeonGoldilocks) -> NeonGoldilocks {
    let mut res = NeonGoldilocks::zero();
    for i in 0..4 {
        unsafe {
            // Extract values
            let a_vals = [vgetq_lane_u64(a.v[i], 0), vgetq_lane_u64(a.v[i], 1)];
            let b_vals = [vgetq_lane_u64(b.v[i], 0), vgetq_lane_u64(b.v[i], 1)];

            // Perform subtraction and modular reduction
            let mut res_vals = [0u64; 2];
            for j in 0..2 {
                let diff = if a_vals[j] >= b_vals[j] {
                    a_vals[j] - b_vals[j]
                } else {
                    GOLDILOCKS_MOD - (b_vals[j] - a_vals[j])
                };
                res_vals[j] = diff;
            }

            // Pack results back
            res.v[i] = vld1q_u64(res_vals.as_ptr());
        }
    }
    res
}

#[inline]
fn mul_internal(a: &NeonGoldilocks, b: &NeonGoldilocks) -> NeonGoldilocks {
    let mut res = NeonGoldilocks::zero();
    for i in 0..4 {
        unsafe {
            // Extract values to perform multiplication
            let a_vals = [vgetq_lane_u64(a.v[i], 0), vgetq_lane_u64(a.v[i], 1)];
            let b_vals = [vgetq_lane_u64(b.v[i], 0), vgetq_lane_u64(b.v[i], 1)];

            // Perform multiplication and modular reduction
            let mut res_vals = [0u64; 2];
            for j in 0..2 {
                let prod = a_vals[j].wrapping_mul(b_vals[j]);
                let prod_hi = (prod >> 32) & 0xFFFFFFFF;
                let prod_lo = prod & 0xFFFFFFFF;
                let t = prod_lo.wrapping_sub(prod_hi.wrapping_mul(GOLDILOCKS_MOD));
                res_vals[j] = if t >= GOLDILOCKS_MOD {
                    t - GOLDILOCKS_MOD
                } else {
                    t
                };
            }

            // Pack results back into NEON vector
            res.v[i] = vld1q_u64(res_vals.as_ptr());
        }
    }
    res
}

impl Hash for NeonGoldilocks {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        unsafe {
            state.write(transmute::<[uint64x2_t; 4], [u8; 64]>(self.v).as_ref());
        }
    }
}
