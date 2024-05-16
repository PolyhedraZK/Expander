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

#[derive(Debug, Clone, Copy, Default)]
pub struct PackedM31 {}

#[derive(Debug, Clone, Copy, Default)]
pub struct VectorizedM31 {}

impl VectorizedM31 {
    pub fn zero() -> Self {
        VectorizedM31 {} // TODO
    }

    pub fn random_bool() -> Self {
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
