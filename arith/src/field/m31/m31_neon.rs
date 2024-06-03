use std::{
    arch::aarch64::*,
    fmt::Debug,
    iter::{Product, Sum},
    mem::{size_of, transmute},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use crate::{Field, M31, M31_MOD};

type PackedDataType = uint32x4_t;
pub const M31_PACK_SIZE: usize = 4;
pub(super) const M31_VECTORIZE_SIZE: usize = 2;

const PACKED_MOD: uint32x4_t = unsafe { transmute([M31_MOD as u32; 4]) };
const PACKED_0: uint32x4_t = unsafe { transmute([0; 4]) };
pub(crate) const PACKED_INV_2: uint32x4_t = unsafe { transmute([1 << 30; 4]) };

use rand::{Rng, RngCore};

#[inline(always)]
fn reduce_sum(x: uint32x4_t) -> uint32x4_t {
    //aarch64 only
    unsafe { vminq_u32(x, vsubq_u32(x, PACKED_MOD)) }
}

#[derive(Clone, Copy)]
pub struct PackedM31 {
    pub v: PackedDataType,
}

impl PackedM31 {
    #[inline(always)]
    pub fn pack_full(x: M31) -> PackedM31 {
        PackedM31 {
            v: unsafe { vdupq_n_u32(x.v) },
        }
    }
}

impl Field for PackedM31 {
    const NAME: &'static str = "Neon Packed Mersenne 31";

    const SIZE: usize = size_of::<PackedDataType>();

    const INV_2: Self = Self { v: PACKED_INV_2 };

    type BaseField = M31;

    #[inline(always)]
    fn zero() -> Self {
        PackedM31 {
            v: unsafe { vdupq_n_u32(0) },
        }
    }

    #[inline(always)]
    fn one() -> Self {
        PackedM31 {
            v: unsafe { vdupq_n_u32(1) },
        }
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl RngCore) -> Self {
        // Caution: this may not produce uniformly random elements
        unsafe {
            PackedM31 {
                v: vld1q_u32(
                    [
                        rng.gen::<u32>() % M31_MOD as u32,
                        rng.gen::<u32>() % M31_MOD as u32,
                        rng.gen::<u32>() % M31_MOD as u32,
                        rng.gen::<u32>() % M31_MOD as u32,
                    ]
                    .as_ptr(),
                ),
            }
        }
    }

    #[inline(always)]
    fn random_bool_unsafe(mut rng: impl RngCore) -> Self {
        unsafe {
            PackedM31 {
                v: vld1q_u32(
                    [
                        rng.gen::<bool>() as u32,
                        rng.gen::<bool>() as u32,
                        rng.gen::<bool>() as u32,
                        rng.gen::<bool>() as u32,
                    ]
                    .as_ptr(),
                ),
            }
        }
    }

    fn exp(&self, _exponent: &Self) -> Self {
        todo!()
    }

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        todo!();
    }

    #[inline(always)]
    fn add_base_elem(&self, _rhs: &Self::BaseField) -> Self {
        unimplemented!()
    }

    #[inline(always)]
    fn add_assign_base_elem(&mut self, _rhs: &Self::BaseField) {
        unimplemented!()
    }

    #[inline(always)]
    fn mul_base_elem(&self, rhs: &Self::BaseField) -> Self {
        *self * rhs
    }

    #[inline(always)]
    fn mul_assign_base_elem(&mut self, rhs: &Self::BaseField) {
        *self = *self * rhs;
    }

    fn as_u32_unchecked(&self) -> u32 {
        unimplemented!("self is a vector, cannot convert to u32")
    }
    fn from_uniform_bytes(_bytes: &[u8; 32]) -> Self {
        unimplemented!(" cannot convert 32 bytes into a vectorized M31")
    }
}

impl Debug for PackedM31 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let data = [
                vgetq_lane_u32(self.v, 0),
                vgetq_lane_u32(self.v, 1),
                vgetq_lane_u32(self.v, 2),
                vgetq_lane_u32(self.v, 3),
            ];
            // if all data is the same, print only one
            if data.iter().all(|&x| x == data[0]) {
                write!(
                    f,
                    "uint32x4_t<8 x {}>",
                    if M31_MOD as u32 - data[0] > 1024 {
                        format!("{}", data[0])
                    } else {
                        format!("-{}", M31_MOD as u32 - data[0])
                    }
                )
            } else {
                write!(f, "uint32x4_t<{:?}>", data)
            }
        }
    }
}

impl Default for PackedM31 {
    fn default() -> Self {
        PackedM31::zero()
    }
}

impl PartialEq for PackedM31 {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            let eq_v = vceqq_u32(self.v, other.v);
            vgetq_lane_u32(eq_v, 0) != 0
                && vgetq_lane_u32(eq_v, 1) != 0
                && vgetq_lane_u32(eq_v, 2) != 0
                && vgetq_lane_u32(eq_v, 3) != 0
        }
    }
}

impl Mul<&PackedM31> for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    fn mul(self, rhs: &PackedM31) -> Self::Output {
        unsafe {
            let prod_hi = vreinterpretq_u32_s32(vqdmulhq_s32(
                vreinterpretq_s32_u32(self.v),
                vreinterpretq_s32_u32(rhs.v),
            ));
            let prod_lo = vmulq_u32(self.v, rhs.v);
            let t = vmlsq_u32(prod_lo, prod_hi, PACKED_MOD);
            PackedM31 { v: reduce_sum(t) }
        }
    }
}

impl Mul for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn mul(self, rhs: PackedM31) -> Self::Output {
        self * &rhs
    }
}

impl Mul<&M31> for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    fn mul(self, rhs: &M31) -> Self::Output {
        let rhs_p = PackedM31::pack_full(*rhs);
        self * rhs_p
    }
}

impl Mul<M31> for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    fn mul(self, rhs: M31) -> Self::Output {
        self * &rhs
    }
}

impl MulAssign<&PackedM31> for PackedM31 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: &PackedM31) {
        *self = *self * rhs;
    }
}

impl MulAssign for PackedM31 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        *self *= &rhs;
    }
}

impl<T: ::core::borrow::Borrow<PackedM31>> Product<T> for PackedM31 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Add<&PackedM31> for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    fn add(self, rhs: &PackedM31) -> Self::Output {
        unsafe {
            PackedM31 {
                v: reduce_sum(vaddq_u32(self.v, rhs.v)),
            }
        }
    }
}

impl Add for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn add(self, rhs: PackedM31) -> Self::Output {
        self + &rhs
    }
}

impl AddAssign<&PackedM31> for PackedM31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &PackedM31) {
        *self = *self + rhs;
    }
}

impl AddAssign for PackedM31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl<T: ::core::borrow::Borrow<PackedM31>> Sum<T> for PackedM31 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl From<u32> for PackedM31 {
    #[inline(always)]
    fn from(x: u32) -> Self {
        PackedM31::pack_full(M31::from(x))
    }
}

impl Neg for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        PackedM31 { v: PACKED_0 } - self
    }
}

impl Sub<&PackedM31> for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    fn sub(self, rhs: &PackedM31) -> Self::Output {
        PackedM31 {
            v: unsafe {
                let diff = vsubq_u32(self.v, rhs.v);
                let u = vaddq_u32(diff, PACKED_MOD);
                vminq_u32(diff, u)
            },
        }
    }
}

impl Sub for PackedM31 {
    type Output = PackedM31;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn sub(self, rhs: PackedM31) -> Self::Output {
        self - &rhs
    }
}

impl SubAssign<&PackedM31> for PackedM31 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: &PackedM31) {
        *self = *self - rhs;
    }
}

impl SubAssign for PackedM31 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}
