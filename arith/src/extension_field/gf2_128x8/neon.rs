use std::arch::aarch64::uint32x4_t;
use std::iter::{Product, Sum};
use std::mem::transmute;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::SimdField;
use crate::{
    field_common,
    neon::{gfadd, gfmul, NeonGF2_128},
    Field, FieldSerde,
};

#[derive(Clone, Copy, Debug)]
pub struct NeonGF2_128x8 {
    v: [uint32x4_t; 8],
}

field_common!(NeonGF2_128x8);

impl Default for NeonGF2_128x8 {
    fn default() -> Self {
        Self::zero()
    }
}

impl PartialEq for NeonGF2_128x8 {
    fn eq(&self, other: &Self) -> bool {
        self.v.iter().zip(other.v.iter()).all(|(a, b)| unsafe {
            transmute::<uint32x4_t, [u8; 16]>(*a) == transmute::<uint32x4_t, [u8; 16]>(*b)
        })
    }
}

impl FieldSerde for NeonGF2_128x8 {
    #[inline(always)]
    fn serialize_into<W: std::io::Write>(&self, mut writer: W) {
        self.v.iter().for_each(|&vv| {
            writer
                .write_all(unsafe { transmute::<uint32x4_t, [u8; 16]>(vv) }.as_ref())
                .unwrap()
        })
    }

    #[inline(always)]
    fn serialized_size() -> usize {
        16 * 8
    }

    #[inline(always)]
    fn deserialize_from<R: std::io::Read>(mut reader: R) -> Self {
        let mut res = Self::zero();
        res.v.iter_mut().for_each(|vv| {
            let mut u = [0u8; 16];
            reader.read_exact(&mut u).unwrap();
            *vv = unsafe { transmute::<[u8; 16], uint32x4_t>(u) }
        });
        res
    }

    #[inline]
    fn try_deserialize_from_ecc_format<R: std::io::Read>(
        mut _reader: R,
    ) -> std::result::Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        unimplemented!("We don't have a serialization for gf2_128 in ecc yet.")

        // let mut res = Self::zero();
        // res.v.iter_mut().for_each(|vv| {
        //     let mut u = [0u8; 32];
        //     reader.read_exact(&mut u).unwrap();
        //     *vv = unsafe { transmute::<[u8; 16], uint32x4_t>(u[..16].try_into().unwrap()) };
        // });
        // Ok(res)
    }
}

impl Field for NeonGF2_128x8 {
    const NAME: &'static str = "Neon Galios Field 2 128x8";

    const SIZE: usize = 16 * 8;

    const FIELD_SIZE: usize = 128 * 8; // in bits

    const ZERO: Self = NeonGF2_128x8 {
        v: [unsafe { transmute::<[u32; 4], uint32x4_t>([0, 0, 0, 0]) }; 8],
    };

    const INV_2: Self = NeonGF2_128x8 {
        v: [unsafe { transmute::<[u32; 4], uint32x4_t>([0, 0, 0, 0]) }; 8],
    }; // should not be used

    #[inline(always)]
    fn zero() -> Self {
        NeonGF2_128x8 {
            v: [unsafe { transmute::<[u32; 4], uint32x4_t>([0, 0, 0, 0]) }; 8],
        }
    }

    #[inline(always)]
    fn one() -> Self {
        NeonGF2_128x8 {
            v: [unsafe { transmute::<[u32; 4], uint32x4_t>([1, 0, 0, 0]) }; 8],
        }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.v
            .iter()
            .all(|vv| unsafe { transmute::<uint32x4_t, [u8; 16]>(*vv) == [0; 16] })
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        NeonGF2_128x8 {
            v: [
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
                unsafe { transmute::<[u64; 2], uint32x4_t>([rng.next_u64(), rng.next_u64()]) },
            ],
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        NeonGF2_128x8 {
            v: [
                unsafe { transmute::<[u32; 4], uint32x4_t>([rng.next_u32() % 2, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([rng.next_u32() % 2, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([rng.next_u32() % 2, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([rng.next_u32() % 2, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([rng.next_u32() % 2, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([rng.next_u32() % 2, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([rng.next_u32() % 2, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([rng.next_u32() % 2, 0, 0, 0]) },
            ],
        }
    }

    #[inline(always)]
    fn exp(&self, exponent: u128) -> Self {
        let mut e = exponent;
        let mut res = Self::one();
        let mut t = *self;
        while e > 0 {
            if e & 1 == 1 {
                res *= t;
            }
            t = t * t;
            e >>= 1;
        }
        res
    }

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        todo!()
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
    fn from_uniform_bytes(_bytes: &[u8; 32]) -> Self {
        unimplemented!("from_uniform_bytes for GF128 doesn't make sense")
    }
}

impl SimdField for NeonGF2_128x8 {
    type Scalar = NeonGF2_128;

    #[inline(always)]
    fn scale(&self, challenge: &Self::Scalar) -> Self {
        NeonGF2_128x8 {
            v: [
                unsafe { gfmul(self.v[0], challenge.v) },
                unsafe { gfmul(self.v[1], challenge.v) },
                unsafe { gfmul(self.v[2], challenge.v) },
                unsafe { gfmul(self.v[3], challenge.v) },
                unsafe { gfmul(self.v[4], challenge.v) },
                unsafe { gfmul(self.v[5], challenge.v) },
                unsafe { gfmul(self.v[6], challenge.v) },
                unsafe { gfmul(self.v[7], challenge.v) },
            ],
        }
    }
    #[inline(always)]
    fn pack_size() -> usize {
        8
    }
}

impl From<NeonGF2_128> for NeonGF2_128x8 {
    fn from(v: NeonGF2_128) -> Self {
        unsafe {
            NeonGF2_128x8 {
                v: [
                    v.v,
                    transmute::<u128, uint32x4_t>(0u128),
                    transmute::<u128, uint32x4_t>(0u128),
                    transmute::<u128, uint32x4_t>(0u128),
                    transmute::<u128, uint32x4_t>(0u128),
                    transmute::<u128, uint32x4_t>(0u128),
                    transmute::<u128, uint32x4_t>(0u128),
                    transmute::<u128, uint32x4_t>(0u128),
                ],
            }
        }
    }
}

impl Neg for NeonGF2_128x8 {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        self
    }
}

impl From<u32> for NeonGF2_128x8 {
    fn from(v: u32) -> Self {
        NeonGF2_128x8 {
            v: [
                unsafe { transmute::<[u32; 4], uint32x4_t>([v, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([v, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([v, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([v, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([v, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([v, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([v, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([v, 0, 0, 0]) },
            ],
        }
    }
}

#[inline(always)]
fn add_internal(a: &NeonGF2_128x8, b: &NeonGF2_128x8) -> NeonGF2_128x8 {
    NeonGF2_128x8 {
        v: [
            unsafe { gfadd(a.v[0], b.v[0]) },
            unsafe { gfadd(a.v[1], b.v[1]) },
            unsafe { gfadd(a.v[2], b.v[2]) },
            unsafe { gfadd(a.v[3], b.v[3]) },
            unsafe { gfadd(a.v[4], b.v[4]) },
            unsafe { gfadd(a.v[5], b.v[5]) },
            unsafe { gfadd(a.v[6], b.v[6]) },
            unsafe { gfadd(a.v[7], b.v[7]) },
        ],
    }
}

#[inline(always)]
fn sub_internal(a: &NeonGF2_128x8, b: &NeonGF2_128x8) -> NeonGF2_128x8 {
    add_internal(a, b)
}

#[inline(always)]
fn mul_internal(a: &NeonGF2_128x8, b: &NeonGF2_128x8) -> NeonGF2_128x8 {
    NeonGF2_128x8 {
        v: [
            unsafe { gfmul(a.v[0], b.v[0]) },
            unsafe { gfmul(a.v[1], b.v[1]) },
            unsafe { gfmul(a.v[2], b.v[2]) },
            unsafe { gfmul(a.v[3], b.v[3]) },
            unsafe { gfmul(a.v[4], b.v[4]) },
            unsafe { gfmul(a.v[5], b.v[5]) },
            unsafe { gfmul(a.v[6], b.v[6]) },
            unsafe { gfmul(a.v[7], b.v[7]) },
        ],
    }
}

impl BinomialExtensionField for NeonGF2_128x8 {
    const DEGREE: usize = 128;
    const W: u32 = 0x87;

    type BaseField = GF2x8;

    #[inline(always)]
    fn mul_by_base_field(&self, base: &Self::BaseField) -> Self {
        let v0 = ((base.v >> 7) & 1u8) as u32;
        let v1 = ((base.v >> 6) & 1u8) as u32;
        let v2 = ((base.v >> 5) & 1u8) as u32;
        let v3 = ((base.v >> 4) & 1u8) as u32;
        let v4 = ((base.v >> 3) & 1u8) as u32;
        let v5 = ((base.v >> 2) & 1u8) as u32;
        let v6 = ((base.v >> 1) & 1u8) as u32;
        let v7 = ((base.v >> 0) & 1u8) as u32;

        Self {
            v: [
                gfmul(res.v[0], transmute::<[u32; 4], uint32x4_t>([v0, 0, 0, 0])),
                gfmul(res.v[1], transmute::<[u32; 4], uint32x4_t>([v2, 0, 0, 0])),
                gfmul(res.v[2], transmute::<[u32; 4], uint32x4_t>([v4, 0, 0, 0])),
                gfmul(res.v[3], transmute::<[u32; 4], uint32x4_t>([v6, 0, 0, 0])),
                gfmul(res.v[4], transmute::<[u32; 4], uint32x4_t>([v1, 0, 0, 0])),
                gfmul(res.v[5], transmute::<[u32; 4], uint32x4_t>([v3, 0, 0, 0])),
                gfmul(res.v[6], transmute::<[u32; 4], uint32x4_t>([v5, 0, 0, 0])),
                gfmul(res.v[7], transmute::<[u32; 4], uint32x4_t>([v7, 0, 0, 0])),
            ],
        }
    }

    #[inline(always)]
    fn add_by_base_field(&self, base: &Self::BaseField) -> Self {
        let v0 = ((base.v >> 7) & 1u8) as u32;
        let v1 = ((base.v >> 6) & 1u8) as u32;
        let v2 = ((base.v >> 5) & 1u8) as u32;
        let v3 = ((base.v >> 4) & 1u8) as u32;
        let v4 = ((base.v >> 3) & 1u8) as u32;
        let v5 = ((base.v >> 2) & 1u8) as u32;
        let v6 = ((base.v >> 1) & 1u8) as u32;
        let v7 = ((base.v >> 0) & 1u8) as u32;

        Self {
            v: [
                unsafe { gfadd(res.v[0], transmute::<[u32; 4], uint32x4_t>([v0, 0, 0, 0])) },
                unsafe { gfadd(res.v[1], transmute::<[u32; 4], uint32x4_t>([v2, 0, 0, 0])) },
                unsafe { gfadd(res.v[2], transmute::<[u32; 4], uint32x4_t>([v4, 0, 0, 0])) },
                unsafe { gfadd(res.v[3], transmute::<[u32; 4], uint32x4_t>([v6, 0, 0, 0])) },
                unsafe { gfadd(res.v[4], transmute::<[u32; 4], uint32x4_t>([v1, 0, 0, 0])) },
                unsafe { gfadd(res.v[5], transmute::<[u32; 4], uint32x4_t>([v3, 0, 0, 0])) },
                unsafe { gfadd(res.v[6], transmute::<[u32; 4], uint32x4_t>([v5, 0, 0, 0])) },
                unsafe { gfadd(res.v[7], transmute::<[u32; 4], uint32x4_t>([v7, 0, 0, 0])) },
            ],
        }
    }
}

impl From<GF2x8> for NeonGF2_128x8 {
    #[inline(always)]
    fn from(v: GF2x8) -> Self {
        let v0 = ((v.v >> 7) & 1u8) as u32;
        let v1 = ((v.v >> 6) & 1u8) as u32;
        let v2 = ((v.v >> 5) & 1u8) as u32;
        let v3 = ((v.v >> 4) & 1u8) as u32;
        let v4 = ((v.v >> 3) & 1u8) as u32;
        let v5 = ((v.v >> 2) & 1u8) as u32;
        let v6 = ((v.v >> 1) & 1u8) as u32;
        let v7 = ((v.v >> 0) & 1u8) as u32;

        NeonGF2_128x8 {
            v: [
                unsafe { transmute::<[u32; 4], uint32x4_t>([v0, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([v2, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([v4, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([v6, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([v1, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([v3, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([v5, 0, 0, 0]) },
                unsafe { transmute::<[u32; 4], uint32x4_t>([v7, 0, 0, 0]) },
            ],
        }
    }
}

impl Mul<GF2> for NeonGF2_128x8 {
    type Output = NeonGF2_128x8;

    #[inline(always)]
    fn mul(self, rhs: GF2) -> Self::Output {
        if rhs.is_zero() {
            Self::zero()
        } else {
            self
        }
    }
}

impl Add<GF2> for NeonGF2_128x8 {
    type Output = NeonGF2_128x8;
    #[inline(always)]
    fn add(self, rhs: GF2) -> Self::Output {
        let rhs_extended = unsafe { transmute::<[u32; 4], uint32x4_t>([rhs.v, 0, 0, 0]) };
        NeonGF2_128x8 {
            v: [
                unsafe { gfmul(self.v[0], rhs_extended) },
                unsafe { gfmul(self.v[1], rhs_extended) },
                unsafe { gfmul(self.v[2], rhs_extended) },
                unsafe { gfmul(self.v[3], rhs_extended) },
                unsafe { gfmul(self.v[4], rhs_extended) },
                unsafe { gfmul(self.v[5], rhs_extended) },
                unsafe { gfmul(self.v[6], rhs_extended) },
                unsafe { gfmul(self.v[7], rhs_extended) },
            ],
        }
    }
}
