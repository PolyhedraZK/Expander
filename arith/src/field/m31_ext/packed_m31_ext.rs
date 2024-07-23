use std::{
    iter::{Product, Sum},
    mem::transmute,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use rand::RngCore;

use crate::{Field, FieldSerde, PackedM31, VectorizedField, M31};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PackedM31Ext3 {
    pub v: [PackedM31; 3],
}

impl FieldSerde for PackedM31Ext3 {
    #[inline(always)]
    fn serialize_into(&self, buffer: &mut [u8]) {
        self.v[0].serialize_into(buffer);
        self.v[1].serialize_into(&mut buffer[32..]);
        self.v[2].serialize_into(&mut buffer[64..]);
    }

    #[inline(always)]
    fn deserialize_from(buffer: &[u8]) -> Self {
        PackedM31Ext3 {
            v: [
                PackedM31::deserialize_from(&buffer[..32]),
                PackedM31::deserialize_from(&buffer[32..64]),
                PackedM31::deserialize_from(&buffer[64..]),
            ],
        }
    }

    fn deserialize_from_ecc_format(_bytes: &[u8; 32]) -> Self {
        todo!()
    }
}

impl VectorizedField for PackedM31Ext3 {
    const PACK_SIZE: usize = 8;

    const VECTORIZE_SIZE: usize = 1;

    type PackedBaseField = PackedM31Ext3;

    #[inline(always)]
    fn as_packed_slices(&self) -> &[PackedM31Ext3] {
        todo!()
    }

    #[inline(always)]
    fn mut_packed_slices(&mut self) -> &mut [Self::PackedBaseField] {
        todo!()
    }
}

impl Field for PackedM31Ext3 {
    const NAME: &'static str = "AVX Packed Mersenne 31 Extension 3";

    const SIZE: usize = 72;
    const INV_2: Self = todo!();

    type BaseField = PackedM31;

    #[inline(always)]
    fn zero() -> Self {
        PackedM31Ext3 {
            v: [PackedM31::zero(); 3],
        }
    }

    #[inline(always)]
    fn one() -> Self {
        PackedM31Ext3 {
            v: [PackedM31::one(), PackedM31::zero(), PackedM31::zero()],
        }
    }

    fn random_unsafe(mut rng: impl RngCore) -> Self {
        PackedM31Ext3 {
            v: [
                PackedM31::random_unsafe(&mut rng),
                PackedM31::random_unsafe(&mut rng),
                PackedM31::random_unsafe(&mut rng),
            ],
        }
    }

    fn random_bool_unsafe(mut rng: impl RngCore) -> Self {
        PackedM31Ext3 {
            v: [
                PackedM31::random_bool_unsafe(&mut rng),
                PackedM31::random_bool_unsafe(&mut rng),
                PackedM31::random_bool_unsafe(&mut rng),
            ],
        }
    }

    fn exp(&self, _exponent: &Self) -> Self {
        todo!()
    }

    fn is_zero(&self) -> bool {
        self.v[0].is_zero() && self.v[1].is_zero() && self.v[2].is_zero()
    }

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        unimplemented!()
    }

    #[inline(always)]
    fn add_base_elem(&self, rhs: &Self::BaseField) -> Self {
        let mut res = *self;
        res.v[0] += rhs;
        res
    }

    #[inline(always)]
    fn add_assign_base_elem(&mut self, rhs: &Self::BaseField) {
        *self = self.add_base_elem(rhs);
    }

    /// Squaring
    #[inline(always)]
    fn square(&self) -> Self {
        let mut res = [PackedM31::default(); 3];
        res[0] =
            self.v[0] * self.v[0] + PackedM31::pack_full(M31 { v: 10 }) * (self.v[1] * self.v[2]);
        res[1] = self.v[0] * self.v[1].double()
            + PackedM31::pack_full(M31 { v: 5 }) * self.v[2] * self.v[2];
        res[2] = self.v[0] * self.v[2].double() + self.v[1] * self.v[1];
        Self { v: res }
    }

    #[inline(always)]
    fn mul_base_elem(&self, rhs: &Self::BaseField) -> Self {
        Self {
            v: [self.v[0] * rhs, self.v[1] * rhs, self.v[2] * rhs],
        }
    }

    #[inline(always)]
    fn mul_assign_base_elem(&mut self, rhs: &Self::BaseField) {
        *self = self.mul_base_elem(rhs);
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        unimplemented!("not supported")
    }

    fn from_uniform_bytes(_bytes: &[u8; 32]) -> Self {
        unimplemented!(" cannot convert 32 bytes into a vectorized M31")
    }
}

// ====================================
// Arithmetics for M31Ext
// ====================================

impl Mul<&PackedM31Ext3> for PackedM31Ext3 {
    type Output = PackedM31Ext3;
    #[inline(always)]
    fn mul(self, rhs: &PackedM31Ext3) -> Self::Output {
        Self {
            v: mul_internal(&self.v, &rhs.v),
        }
    }
}

impl Mul for PackedM31Ext3 {
    type Output = PackedM31Ext3;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn mul(self, rhs: PackedM31Ext3) -> Self::Output {
        self * &rhs
    }
}

impl MulAssign<&PackedM31Ext3> for PackedM31Ext3 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: &PackedM31Ext3) {
        *self = *self * rhs;
    }
}

impl MulAssign for PackedM31Ext3 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        *self *= &rhs;
    }
}

impl<T: ::core::borrow::Borrow<PackedM31Ext3>> Product<T> for PackedM31Ext3 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Add<&PackedM31Ext3> for PackedM31Ext3 {
    type Output = PackedM31Ext3;
    #[inline(always)]
    fn add(self, rhs: &PackedM31Ext3) -> Self::Output {
        let mut vv = self.v;
        vv[0] += rhs.v[0];
        vv[1] += rhs.v[1];
        vv[2] += rhs.v[2];

        PackedM31Ext3 { v: vv }
    }
}

impl Add for PackedM31Ext3 {
    type Output = PackedM31Ext3;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn add(self, rhs: PackedM31Ext3) -> Self::Output {
        self + &rhs
    }
}

impl AddAssign<&PackedM31Ext3> for PackedM31Ext3 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &PackedM31Ext3) {
        *self = *self + rhs;
    }
}

impl AddAssign for PackedM31Ext3 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl<T: ::core::borrow::Borrow<PackedM31Ext3>> Sum<T> for PackedM31Ext3 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl Neg for PackedM31Ext3 {
    type Output = PackedM31Ext3;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        PackedM31Ext3 {
            v: [-self.v[0], -self.v[1], -self.v[2]],
        }
    }
}

impl Sub<&PackedM31Ext3> for PackedM31Ext3 {
    type Output = PackedM31Ext3;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn sub(self, rhs: &PackedM31Ext3) -> Self::Output {
        self + &(-*rhs)
    }
}

impl Sub for PackedM31Ext3 {
    type Output = PackedM31Ext3;
    #[inline(always)]
    #[allow(clippy::op_ref)]
    fn sub(self, rhs: PackedM31Ext3) -> Self::Output {
        self - &rhs
    }
}

impl SubAssign<&PackedM31Ext3> for PackedM31Ext3 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: &PackedM31Ext3) {
        *self = *self - rhs;
    }
}

impl SubAssign for PackedM31Ext3 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}

impl From<u32> for PackedM31Ext3 {
    #[inline(always)]
    fn from(x: u32) -> Self {
        PackedM31Ext3 {
            v: [PackedM31::from(x), PackedM31::zero(), PackedM31::zero()],
        }
    }
}

const FIVE: PackedM31 = PackedM31 {
    v: unsafe { transmute::<[i32; 8], std::arch::x86_64::__m256i>([5; 8]) },
};

// polynomial mod (x^3 - 5)
//
//   (a0 + a1*x + a2*x^2) * (b0 + b1*x + b2*x^2) mod (x^3 - 5)
// = a0*b0 + (a0*b1 + a1*b0)*x + (a0*b2 + a1*b1 + a2*b0)*x^2
// + (a1*b2 + a2*b1)*x^3 + a2*b2*x^4 mod (x^3 - 5)
// = a0*b0 + 5*(a1*b2 + a2*b1)
// + (a0*b1 + a1*b0)*x + 5* a2*b2
// + (a0*b2 + a1*b1 + a2*b0)*x^2
fn mul_internal(a: &[PackedM31; 3], b: &[PackedM31; 3]) -> [PackedM31; 3] {
    let mut res = [PackedM31::default(); 3];
    res[0] = a[0] * b[0] + FIVE * (a[1] * b[2] + a[2] * b[1]);
    res[1] = a[0] * b[1] + a[1] * b[0] + FIVE * a[2] * b[2];
    res[2] = a[0] * b[2] + a[1] * b[1] + a[2] * b[0];
    res
}
