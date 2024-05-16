use crate::Field;
use std::ops::{AddAssign, Mul};

pub const M31_MOD: i32 = 2147483647;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct M31 {
    v: usize,
}

impl Field for M31 {
    fn zero() -> Self {
        todo!()
    }

    fn one() -> Self {
        todo!()
    }

    fn random() -> Self {
        todo!()
    }

    fn random_bool() -> Self {
        todo!()
    }

    fn inv(&self) -> Self {
        todo!()
    }
}

impl Mul for M31 {
    type Output = M31;
    fn mul(self, rhs: M31) -> Self::Output {
        todo!()
    }
}

impl Mul<&M31> for M31 {
    type Output = M31;
    fn mul(self, rhs: &M31) -> Self::Output {
        M31 {
            v: (self.v * rhs.v) % M31_MOD as usize,
        }
    }
}

impl AddAssign<&M31> for M31 {
    fn add_assign(&mut self, rhs: &M31) {
        todo!()
    }
}

impl AddAssign for M31 {
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl From<usize> for M31 {
    fn from(x: usize) -> Self {
        M31 {
            v: if x < M31_MOD as usize {
                x
            } else {
                x % M31_MOD as usize
            },
        }
    }
}

#[cfg(target_arch = "x86_64")]
pub mod m31_avx;
#[cfg(target_arch = "x86_64")]
pub use m31_avx::{PackedM31, M31_PACK_SIZE, M31_VECTORIZE_SIZE};

#[cfg(target_arch = "arm")]
pub mod m31_neon;
#[cfg(target_arch = "arm")]
pub use m31_avx::{PackedM31, M31_PACK_SIZE, M31_VECTORIZE_SIZE};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VectorizedM31 {
    v: [PackedM31; M31_VECTORIZE_SIZE],
}

impl Field for VectorizedM31 {
    fn zero() -> Self {
        VectorizedM31 {
            v: [PackedM31::zero(); M31_VECTORIZE_SIZE],
        }
    }

    fn one() -> Self {
        todo!()
    }

    fn random() -> Self {
        VectorizedM31 {
            v: (0..M31_VECTORIZE_SIZE)
                .map(|_| PackedM31::random())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }

    fn random_bool() -> Self {
        VectorizedM31 {
            v: (0..M31_VECTORIZE_SIZE)
                .map(|_| PackedM31::random_bool())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }

    fn inv(&self) -> Self {
        todo!()
    }
}

impl Mul<&VectorizedM31> for VectorizedM31 {
    type Output = VectorizedM31;
    fn mul(self, rhs: &VectorizedM31) -> Self::Output {
        VectorizedM31 {
            v: self
                .v
                .iter()
                .zip(rhs.v.iter())
                .map(|(a, b)| *a * b)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }
}

impl Mul for VectorizedM31 {
    type Output = VectorizedM31;
    fn mul(self, rhs: VectorizedM31) -> Self::Output {
        self * &rhs
    }
}

impl Mul<M31> for VectorizedM31 {
    type Output = VectorizedM31;
    fn mul(self, rhs: M31) -> Self::Output {
        VectorizedM31 {
            v: self
                .v
                .iter()
                .map(|x| *x * PackedM31::pack_full(rhs))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }
}

impl AddAssign<&VectorizedM31> for VectorizedM31 {
    fn add_assign(&mut self, rhs: &VectorizedM31) {
        self.v
            .iter_mut()
            .zip(rhs.v.iter())
            .for_each(|(a, b)| *a += b);
    }
}

impl AddAssign for VectorizedM31 {
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl From<usize> for VectorizedM31 {
    fn from(x: usize) -> Self {
        VectorizedM31 {
            v: [PackedM31::from(x); M31_VECTORIZE_SIZE],
        }
    }
}
