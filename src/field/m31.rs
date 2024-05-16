use crate::Field;
use std::ops::{AddAssign, Mul};

pub const M31_MOD: i32 = 2147483647;

#[cfg(target_arch = "x86_64")]
pub mod m31_avx;
#[cfg(target_arch = "x86_64")]
pub use m31_avx::*;

#[cfg(target_arch = "arm")]
pub mod m31_neon;
#[cfg(target_arch = "arm")]
pub use m31_neon::*;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PackedM31 {}

impl Mul for PackedM31 {
    type Output = PackedM31;
    fn mul(self, rhs: PackedM31) -> Self::Output {
        PackedM31 {} // TODO
    }
}

impl AddAssign for PackedM31 {
    fn add_assign(&mut self, rhs: Self) {
        // TODO
    }
}

impl From<usize> for PackedM31 {
    fn from(x: usize) -> Self {
        PackedM31 {} // TODO
    }
}

impl Field for PackedM31 {
    fn zero() -> Self {
        PackedM31 {} // TODO
    }

    fn one() -> Self {
        PackedM31 {} // TODO
    }

    fn random() -> Self {
        PackedM31 {} // TODO
    }

    fn random_bool() -> Self {
        todo!()
    }

    fn inv(&self) -> Self {
        PackedM31 {} // TODO
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VectorizedM31 {}

impl Field for VectorizedM31 {
    fn zero() -> Self {
        VectorizedM31 {} // TODO
    }

    fn one() -> Self {
        VectorizedM31 {} // TODO
    }

    fn random() -> Self {
        VectorizedM31 {} // TODO
    }

    fn random_bool() -> Self {
        todo!()
    }

    fn inv(&self) -> Self {
        VectorizedM31 {} // TODO
    }
}

impl Mul for VectorizedM31 {
    type Output = VectorizedM31;
    fn mul(self, rhs: VectorizedM31) -> Self::Output {
        VectorizedM31 {} // TODO
    }
}

impl Mul<&VectorizedM31> for VectorizedM31 {
    type Output = VectorizedM31;
    fn mul(self, rhs: &VectorizedM31) -> Self::Output {
        VectorizedM31 {} // TODO
    }
}

impl Mul<M31> for VectorizedM31 {
    type Output = VectorizedM31;
    fn mul(self, rhs: M31) -> Self::Output {
        VectorizedM31 {} // TODO
    }
}

impl AddAssign for VectorizedM31 {
    fn add_assign(&mut self, rhs: Self) {
        // TODO
    }
}

impl From<usize> for VectorizedM31 {
    fn from(x: usize) -> Self {
        VectorizedM31 {} // TODO
    }
}
