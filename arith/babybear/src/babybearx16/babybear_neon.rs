use std::{
    arch::aarch64::*,
    fmt::Debug,
    hash::{Hash, Hasher},
    io::{Read, Write},
    iter::{Product, Sum},
    mem::transmute,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::{field_common, Field, SimdField};
use ethnum::U256;
use rand::RngCore;
use serdes::{ExpSerde, SerdeResult};

use crate::{babybear::BABY_BEAR_MOD, BabyBear};

const BABY_BEAR_PACK_SIZE: usize = 16;

#[derive(Clone, Copy)]
pub struct NeonBabyBear {
    pub v: [uint32x4_t; 4],
}

const PACKED_MOD: uint32x4_t = unsafe { transmute::<[u32; 4], uint32x4_t>([BABY_BEAR_MOD; 4]) };

#[inline]
unsafe fn mod_reduce_epi32(x: uint32x4_t) -> uint32x4_t {
    let mask = vcgeq_u32(x, PACKED_MOD);
    vsubq_u32(x, vandq_u32(mask, PACKED_MOD))
}

field_common!(NeonBabyBear);

impl NeonBabyBear {
    #[inline(always)]
    pub fn pack_full(x: BabyBear) -> NeonBabyBear {
        NeonBabyBear {
            v: unsafe {
                // Safety: memory representation of [x; BABY_BEAR_PACK_SIZE]
                // is 16 u32s, which can be reinterpreted as 4 uint32x4_t.
                transmute([x; BABY_BEAR_PACK_SIZE])
            },
        }
    }
}

impl ExpSerde for NeonBabyBear {
    const SERIALIZED_SIZE: usize = (128 / 8) * 4;

    #[inline(always)]
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        unsafe {
            let data = self
                .v
                .iter()
                .map(|x| mod_reduce_epi32(*x))
                .collect::<Vec<_>>();
            let data = transmute::<[uint32x4_t; 4], [u8; 64]>(data[..4].try_into().unwrap());

            writer.write_all(&data)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let mut data = [0; 64];
        reader.read_exact(&mut data)?;
        unsafe {
            Ok(NeonBabyBear {
                v: transmute::<[u8; 64], [uint32x4_t; 4]>(data),
            })
        }
    }
}

impl Field for NeonBabyBear {
    const NAME: &'static str = "Neon Packed BabyBear";

    const SIZE: usize = 128 / 8 * 4;

    const FIELD_SIZE: usize = 32;

    const ZERO: Self = Self {
        v: unsafe { transmute([BabyBear::ZERO; BABY_BEAR_PACK_SIZE]) },
    };

    const ONE: Self = Self {
        v: unsafe { transmute([BabyBear::ONE; BABY_BEAR_PACK_SIZE]) },
    };

    const INV_2: Self = Self {
        v: unsafe { transmute([BabyBear::INV_2; BABY_BEAR_PACK_SIZE]) },
    };

    const MODULUS: U256 = BabyBear::MODULUS;

    fn zero() -> Self {
        Self::ZERO
    }

    fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }

    fn one() -> Self {
        Self::ONE
    }

    fn random_unsafe(mut rng: impl RngCore) -> Self {
        let mut sample = [BabyBear::ZERO; BABY_BEAR_PACK_SIZE];
        for i in 0..BABY_BEAR_PACK_SIZE {
            sample[i] = BabyBear::random_unsafe(&mut rng);
        }
        Self::pack(&sample)
    }

    fn random_bool(mut rng: impl RngCore) -> Self {
        let sample = (0..BABY_BEAR_PACK_SIZE)
            .map(|_| BabyBear::random_bool(&mut rng))
            .collect::<Vec<_>>();
        Self::pack(&sample)
    }

    fn inv(&self) -> Option<Self> {
        // slow, should not be used in production
        let mut babybear_vec =
            unsafe { transmute::<[uint32x4_t; 4], [BabyBear; BABY_BEAR_PACK_SIZE]>(self.v) };
        let is_nonzero = babybear_vec.iter().all(|x| !x.is_zero());
        if !is_nonzero {
            return None;
        };
        babybear_vec.iter_mut().for_each(|x| *x = x.inv().unwrap());
        Some(Self::pack(&babybear_vec))
    }

    fn as_u32_unchecked(&self) -> u32 {
        unimplemented!("self is a vector, cannot convert to u32")
    }

    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        Self::pack_full(BabyBear::from_uniform_bytes(bytes))
    }
}

impl SimdField for NeonBabyBear {
    type Scalar = BabyBear;

    const PACK_SIZE: usize = BABY_BEAR_PACK_SIZE;

    #[inline]
    fn scale(&self, challenge: &Self::Scalar) -> Self {
        *self * *challenge
    }

    #[inline(always)]
    fn pack(base_vec: &[Self::Scalar]) -> Self {
        debug_assert!(base_vec.len() == BABY_BEAR_PACK_SIZE);
        let ret: [Self::Scalar; BABY_BEAR_PACK_SIZE] = base_vec.try_into().unwrap();
        Self {
            // Transmute is reinterpreting an array of scalars in Montgomery form to an AVX register
            v: unsafe { transmute(ret) },
        }
    }

    #[inline(always)]
    fn unpack(&self) -> Vec<Self::Scalar> {
        // Transmute is reinterpreting an AVX register to an array of scalars in Montgomery form
        let ret =
            unsafe { transmute::<[uint32x4_t; 4], [Self::Scalar; BABY_BEAR_PACK_SIZE]>(self.v) };
        ret.to_vec()
    }
}

impl From<BabyBear> for NeonBabyBear {
    #[inline(always)]
    fn from(x: BabyBear) -> Self {
        NeonBabyBear::pack_full(x)
    }
}

impl Debug for NeonBabyBear {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let unpacked = self.unpack();
        if unpacked.iter().all(|x| *x == unpacked[0]) {
            write!(f, "uint32x4_t<16 x {:?}>", unpacked[0])
        } else {
            write!(f, "uint32x4_t<{unpacked:?}>")
        }
    }
}

impl Default for NeonBabyBear {
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialEq for NeonBabyBear {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            transmute::<[uint32x4_t; 4], [u32; 16]>(
                self.v
                    .iter()
                    .map(|x| mod_reduce_epi32(*x))
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            ) == transmute::<[uint32x4_t; 4], [u32; 16]>(
                other
                    .v
                    .iter()
                    .map(|x| mod_reduce_epi32(*x))
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            )
        }
    }
}

impl Eq for NeonBabyBear {}

impl Mul<&BabyBear> for NeonBabyBear {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: &BabyBear) -> Self::Output {
        self * NeonBabyBear::pack_full(*rhs)
    }
}

impl Mul<BabyBear> for NeonBabyBear {
    type Output = NeonBabyBear;
    #[inline(always)]
    fn mul(self, rhs: BabyBear) -> Self::Output {
        self * &rhs
    }
}

impl Add<BabyBear> for NeonBabyBear {
    type Output = NeonBabyBear;
    #[inline(always)]
    fn add(self, rhs: BabyBear) -> Self::Output {
        self + NeonBabyBear::pack_full(rhs)
    }
}

impl From<u32> for NeonBabyBear {
    #[inline(always)]
    fn from(value: u32) -> Self {
        // BabyBear::new converts to Montgomery form
        NeonBabyBear::pack_full(BabyBear::new(value))
    }
}

impl Neg for NeonBabyBear {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        NeonBabyBear {
            v: [
                p3_instructions::neg(self.v[0]),
                p3_instructions::neg(self.v[1]),
                p3_instructions::neg(self.v[2]),
                p3_instructions::neg(self.v[3]),
            ],
        }
    }
}

#[inline(always)]
fn add_internal(a: &NeonBabyBear, b: &NeonBabyBear) -> NeonBabyBear {
    NeonBabyBear {
        v: [
            p3_instructions::add(a.v[0], b.v[0]),
            p3_instructions::add(a.v[1], b.v[1]),
            p3_instructions::add(a.v[2], b.v[2]),
            p3_instructions::add(a.v[3], b.v[3]),
        ],
    }
}

#[inline(always)]
fn sub_internal(a: &NeonBabyBear, b: &NeonBabyBear) -> NeonBabyBear {
    NeonBabyBear {
        v: [
            p3_instructions::sub(a.v[0], b.v[0]),
            p3_instructions::sub(a.v[1], b.v[1]),
            p3_instructions::sub(a.v[2], b.v[2]),
            p3_instructions::sub(a.v[3], b.v[3]),
        ],
    }
}

#[inline]
fn mul_internal(a: &NeonBabyBear, b: &NeonBabyBear) -> NeonBabyBear {
    NeonBabyBear {
        v: [
            p3_instructions::mul(a.v[0], b.v[0]),
            p3_instructions::mul(a.v[1], b.v[1]),
            p3_instructions::mul(a.v[2], b.v[2]),
            p3_instructions::mul(a.v[3], b.v[3]),
        ],
    }
}

impl Hash for NeonBabyBear {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe {
            state.write(transmute::<[uint32x4_t; 4], [u8; 64]>(self.v).as_ref());
        }
    }
}

mod p3_instructions {
    use std::{arch::aarch64::*, mem::transmute};

    const PACKED_P: uint32x4_t = unsafe { transmute::<[u32; 4], uint32x4_t>([0x7fffffff; 4]) };
    const PACKED_MU: int32x4_t = unsafe { transmute::<[i32; 4], int32x4_t>([-0x77ffffff; 4]) };

    #[inline]
    #[must_use]
    pub(super) fn add(lhs: uint32x4_t, rhs: uint32x4_t) -> uint32x4_t {
        unsafe {
            let t = vaddq_u32(lhs, rhs);
            let u = vsubq_u32(t, PACKED_P);
            vminq_u32(t, u)
        }
    }

    #[inline]
    #[must_use]
    pub(super) fn sub(lhs: uint32x4_t, rhs: uint32x4_t) -> uint32x4_t {
        unsafe {
            let diff = vsubq_u32(lhs, rhs);
            let underflow = vcltq_u32(lhs, rhs);
            vmlsq_u32(diff, underflow, PACKED_P)
        }
    }

    #[inline]
    #[must_use]
    pub(super) fn neg(val: uint32x4_t) -> uint32x4_t {
        unsafe {
            let t = vsubq_u32(PACKED_P, val);
            let is_zero = vceqzq_u32(val);
            vbicq_u32(t, is_zero)
        }
    }

    #[inline]
    #[must_use]
    fn mulby_mu(val: int32x4_t) -> int32x4_t {
        unsafe { vmulq_s32(val, PACKED_MU) }
    }

    #[inline]
    #[must_use]
    fn get_c_hi(lhs: int32x4_t, rhs: int32x4_t) -> int32x4_t {
        unsafe { vqdmulhq_s32(lhs, rhs) }
    }

    #[inline]
    #[must_use]
    fn get_qp_hi(lhs: int32x4_t, mu_rhs: int32x4_t) -> int32x4_t {
        unsafe {
            let q = vmulq_s32(lhs, mu_rhs);
            vqdmulhq_s32(q, vreinterpretq_s32_u32(PACKED_P))
        }
    }

    #[inline]
    #[must_use]
    fn get_reduced_d(c_hi: int32x4_t, qp_hi: int32x4_t) -> uint32x4_t {
        unsafe {
            let d = vreinterpretq_u32_s32(vsubq_s32(c_hi, qp_hi));
            let underflow = vcltq_s32(c_hi, qp_hi);
            vmlsq_u32(d, underflow, PACKED_P)
        }
    }

    #[inline]
    #[must_use]
    pub(super) fn mul(lhs: uint32x4_t, rhs: uint32x4_t) -> uint32x4_t {
        unsafe {
            let lhs = vreinterpretq_s32_u32(lhs);
            let rhs = vreinterpretq_s32_u32(rhs);

            let mu_rhs = mulby_mu(rhs);
            let c_hi = get_c_hi(lhs, rhs);
            let qp_hi = get_qp_hi(lhs, mu_rhs);
            get_reduced_d(c_hi, qp_hi)
        }
    }
}
