use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use arith::{field_common, ExtensionField, Field};
use ethnum::U256;
use gf2::GF2;
use serdes::{ExpSerde, SerdeResult};

#[derive(Clone, Copy, Debug)]
pub struct ScalarGF2_128 {
    pub(crate) v: [u64; 2],
}

field_common!(ScalarGF2_128);

#[inline(always)]
fn clmul(a: u64, b: u64) -> u128 {
    let mut result: u128 = 0;
    for i in 0..64 {
        if (b >> i) & 1 == 1 {
            result ^= (a as u128) << i;
        }
    }
    result
}

#[inline(always)]
pub(crate) fn gfmul_scalars(a: [u64; 2], b: [u64; 2]) -> [u64; 2] {
    let a0 = a[0];
    let a1 = a[1];
    let b0 = b[0];
    let b1 = b[1];

    let tmp3 = clmul(a0, b0);
    let tmp6 = clmul(a1, b1);

    let tmp4 = clmul(a0 ^ a1, b0 ^ b1) ^ tmp3 ^ tmp6;

    let low = tmp3 ^ (tmp4 << 64);
    let high = tmp6 ^ (tmp4 >> 64);

    let low_lo = low as u64;
    let low_hi = (low >> 64) as u64;
    let high_lo = high as u64;
    let high_hi = (high >> 64) as u64;

    let high_u32s: [u32; 4] = [
        high_lo as u32,
        (high_lo >> 32) as u32,
        high_hi as u32,
        (high_hi >> 32) as u32,
    ];

    let mut shifted_31 = [0u32; 4];
    let mut shifted_30 = [0u32; 4];
    let mut shifted_25 = [0u32; 4];
    for i in 0..4 {
        shifted_31[i] = high_u32s[i] >> 31;
        shifted_30[i] = high_u32s[i] >> 30;
        shifted_25[i] = high_u32s[i] >> 25;
    }

    let mut tmp7 = [0u32; 4];
    for i in 0..4 {
        tmp7[i] = shifted_31[i] ^ shifted_30[i] ^ shifted_25[i];
    }

    let rotated = [tmp7[3], tmp7[0], tmp7[1], tmp7[2]];

    let mask_val = [rotated[0], 0u32, 0u32, 0u32];
    let not_mask_val = [0u32, rotated[1], rotated[2], rotated[3]];

    let low_u32s: [u32; 4] = [
        low_lo as u32,
        (low_lo >> 32) as u32,
        low_hi as u32,
        (low_hi >> 32) as u32,
    ];

    let mut low_result = [0u32; 4];
    let mut high_result = [0u32; 4];
    for i in 0..4 {
        low_result[i] = low_u32s[i] ^ not_mask_val[i];
        high_result[i] = high_u32s[i] ^ mask_val[i];
    }

    let mut shl1 = [0u32; 4];
    let mut shl2 = [0u32; 4];
    let mut shl7 = [0u32; 4];
    for i in 0..4 {
        shl1[i] = high_result[i] << 1;
        shl2[i] = high_result[i] << 2;
        shl7[i] = high_result[i] << 7;
    }

    for i in 0..4 {
        low_result[i] ^= shl1[i] ^ shl2[i] ^ shl7[i] ^ high_result[i];
    }

    let r0 = (low_result[0] as u64) | ((low_result[1] as u64) << 32);
    let r1 = (low_result[2] as u64) | ((low_result[3] as u64) << 32);

    [r0, r1]
}

#[inline(always)]
pub(crate) fn mul_by_x_scalar(a: &[u64; 2]) -> [u64; 2] {
    let high_bit = a[1] >> 63;
    let shifted_lo = a[0] << 1;
    let shifted_hi = (a[1] << 1) | (a[0] >> 63);
    let reduction = 0x87 * high_bit;
    [shifted_lo ^ reduction, shifted_hi]
}

#[inline(always)]
fn add_internal(a: &ScalarGF2_128, b: &ScalarGF2_128) -> ScalarGF2_128 {
    ScalarGF2_128 {
        v: [a.v[0] ^ b.v[0], a.v[1] ^ b.v[1]],
    }
}

#[inline(always)]
fn sub_internal(a: &ScalarGF2_128, b: &ScalarGF2_128) -> ScalarGF2_128 {
    add_internal(a, b)
}

#[inline(always)]
fn mul_internal(a: &ScalarGF2_128, b: &ScalarGF2_128) -> ScalarGF2_128 {
    ScalarGF2_128 {
        v: gfmul_scalars(a.v, b.v),
    }
}

impl ExpSerde for ScalarGF2_128 {
    #[inline(always)]
    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        writer.write_all(&self.v[0].to_le_bytes())?;
        writer.write_all(&self.v[1].to_le_bytes())?;
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: std::io::Read>(mut reader: R) -> SerdeResult<Self> {
        let mut buf = [0u8; 16];
        reader.read_exact(&mut buf)?;
        Ok(ScalarGF2_128 {
            v: [
                u64::from_le_bytes(buf[0..8].try_into().unwrap()),
                u64::from_le_bytes(buf[8..16].try_into().unwrap()),
            ],
        })
    }
}

impl Field for ScalarGF2_128 {
    const NAME: &'static str = "Scalar GF(2^128)";

    const SIZE: usize = 16;

    const FIELD_SIZE: usize = 128;

    const ZERO: Self = ScalarGF2_128 { v: [0; 2] };

    const ONE: Self = ScalarGF2_128 { v: [1, 0] };

    const INV_2: Self = ScalarGF2_128 { v: [0; 2] };

    const MODULUS: U256 = unimplemented!();

    #[inline(always)]
    fn zero() -> Self {
        ScalarGF2_128 { v: [0; 2] }
    }

    #[inline(always)]
    fn one() -> Self {
        ScalarGF2_128 { v: [1, 0] }
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        ScalarGF2_128 {
            v: [rng.next_u64(), rng.next_u64()],
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        ScalarGF2_128 {
            v: [rng.next_u32() as u64 & 1, 0],
        }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.v == [0; 2]
    }

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }
        let p_m2 = u128::MAX - 1;
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
    fn from_uniform_bytes(bytes: &[u8]) -> Self {
        let buf: [u8; 16] = bytes[..16].try_into().unwrap();
        ScalarGF2_128 {
            v: [
                u64::from_le_bytes(buf[0..8].try_into().unwrap()),
                u64::from_le_bytes(buf[8..16].try_into().unwrap()),
            ],
        }
    }
}

impl ExtensionField for ScalarGF2_128 {
    const DEGREE: usize = 128;

    const W: u32 = 0x87;

    const X: Self = ScalarGF2_128 { v: [2, 0] };

    type BaseField = GF2;

    #[inline(always)]
    fn mul_by_base_field(&self, base: &Self::BaseField) -> Self {
        if base.is_zero() {
            Self::zero()
        } else {
            *self
        }
    }

    #[inline(always)]
    fn add_by_base_field(&self, base: &Self::BaseField) -> Self {
        if base.is_zero() {
            return *self;
        }
        add_internal(&Self::one(), self)
    }

    #[inline(always)]
    fn mul_by_x(&self) -> Self {
        Self {
            v: mul_by_x_scalar(&self.v),
        }
    }

    #[inline(always)]
    fn from_limbs(limbs: &[Self::BaseField]) -> Self {
        let mut local_limbs = limbs.to_vec();
        local_limbs.resize(Self::DEGREE, Self::BaseField::ZERO);

        let mut u32_lanes = [0u32; 4];
        local_limbs
            .chunks(32)
            .zip(u32_lanes.iter_mut())
            .for_each(|(limbs_by_32, u32_lane)| {
                limbs_by_32.iter().enumerate().for_each(|(ith_limb, limb)| {
                    *u32_lane |= (limb.v as u32) << ith_limb;
                });
            });

        ScalarGF2_128 {
            v: [
                (u32_lanes[0] as u64) | ((u32_lanes[1] as u64) << 32),
                (u32_lanes[2] as u64) | ((u32_lanes[3] as u64) << 32),
            ],
        }
    }

    #[inline(always)]
    fn to_limbs(&self) -> Vec<Self::BaseField> {
        let mut u32_extracted = [
            self.v[0] as u32,
            (self.v[0] >> 32) as u32,
            self.v[1] as u32,
            (self.v[1] >> 32) as u32,
        ];

        let mut res = vec![Self::BaseField::ZERO; 128];
        u32_extracted
            .iter_mut()
            .enumerate()
            .for_each(|(ith_u32, u32_lane)| {
                (0..32).for_each(|ith_bit| {
                    let res_index = ith_bit + ith_u32 * 32;
                    res[res_index] = From::from(*u32_lane);
                    *u32_lane >>= 1;
                })
            });

        res
    }
}

impl Mul<GF2> for ScalarGF2_128 {
    type Output = ScalarGF2_128;

    #[inline]
    fn mul(self, rhs: GF2) -> Self::Output {
        self.mul_by_base_field(&rhs)
    }
}

impl From<GF2> for ScalarGF2_128 {
    #[inline(always)]
    fn from(v: GF2) -> Self {
        match v.v {
            0 => Self::zero(),
            1 => Self::one(),
            _ => panic!("Invalid value for GF2"),
        }
    }
}

impl Default for ScalarGF2_128 {
    #[inline(always)]
    fn default() -> Self {
        Self::zero()
    }
}

impl PartialEq for ScalarGF2_128 {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.v == other.v
    }
}

impl Eq for ScalarGF2_128 {}

impl Neg for ScalarGF2_128 {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self {
        self
    }
}

impl From<u32> for ScalarGF2_128 {
    #[inline(always)]
    fn from(v: u32) -> Self {
        ScalarGF2_128 {
            v: [v as u64, 0],
        }
    }
}

impl std::hash::Hash for ScalarGF2_128 {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.v[0].to_le_bytes());
        state.write(&self.v[1].to_le_bytes());
    }
}
