use std::ops::{Add, AddAssign, Mul};

pub const M31_PACK_SIZE: usize = 8;
pub const M31_VECTORIZE_SIZE: usize = 1;

#[derive(Debug, Clone, Copy, Default)]
pub struct M31 {}

impl M31 {
    pub fn zero() -> Self {
        M31 {} // TODO
    }

    pub fn random_bool() -> Self {
        M31 {} // TODO
    }
}

impl From<usize> for M31 {
    fn from(x: usize) -> Self {
        M31 {} // TODO
    }
}

impl Mul for M31 {
    type Output = M31;
    fn mul(self, rhs: M31) -> Self::Output {
        M31 {} // TODO
    }
}

impl Mul<&M31> for M31 {
    type Output = M31;
    fn mul(self, rhs: &M31) -> Self::Output {
        M31 {} // TODO
    }
}

impl AddAssign for M31 {
    fn add_assign(&mut self, rhs: Self) {
        // TODO
    }
}
