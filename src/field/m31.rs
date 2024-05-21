use crate::Field;
use std::{
    mem::{size_of, transmute},
    ops::{Add, AddAssign, Mul, Neg, Sub},
};

pub const M31_MOD: i32 = 2147483647;
fn mod_reduce_int(x: i64) -> i64 {
    (x & M31_MOD as i64) + (x >> 31)
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct M31 {
    pub v: u32,
}

impl M31 {
    pub const SIZE: usize = size_of::<u32>();
    pub const INV_2: M31 = M31 { v: 1 << 30 };
    #[inline(always)]
    pub fn serialize_into(&self, buffer: &mut [u8]) {
        buffer[..M31::SIZE].copy_from_slice(unsafe {
            std::slice::from_raw_parts(&self.v as *const u32 as *const u8, M31::SIZE)
        });
    }
    #[inline(always)]
    pub fn deserialize_from(buffer: &[u8]) -> Self {
        let ptr = buffer.as_ptr() as *const u32;

        let mut v = unsafe { ptr.read_unaligned() } as i64;
        v = mod_reduce_int(v);
        if v >= M31_MOD as i64 {
            v -= M31_MOD as i64;
        }
        M31 { v: v as u32 }
    }
}

impl Field for M31 {
    #[inline(always)]
    fn zero() -> Self {
        M31 { v: 0 }
    }

    #[inline(always)]
    fn one() -> Self {
        M31 { v: 1 }
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

impl Mul<&M31> for M31 {
    type Output = M31;
    #[inline(always)]
    fn mul(self, rhs: &M31) -> Self::Output {
        let mut vv = self.v as i64 * rhs.v as i64;
        vv = mod_reduce_int(vv);
        if vv >= M31_MOD as i64 {
            vv -= M31_MOD as i64;
        }
        M31 { v: vv as u32 }
    }
}

impl Mul for M31 {
    type Output = M31;
    #[inline(always)]
    fn mul(self, rhs: M31) -> Self::Output {
        self * &rhs
    }
}

impl Add<&M31> for M31 {
    type Output = M31;
    #[inline(always)]
    fn add(self, rhs: &M31) -> Self::Output {
        let mut vv = self.v + rhs.v;
        if vv >= M31_MOD as u32 {
            vv -= M31_MOD as u32;
        }
        M31 { v: vv }
    }
}

impl Add for M31 {
    type Output = M31;
    #[inline(always)]
    fn add(self, rhs: M31) -> Self::Output {
        self + &rhs
    }
}

impl Neg for M31 {
    type Output = M31;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        M31 {
            v: if self.v == 0 {
                0
            } else {
                M31_MOD as u32 - self.v
            },
        }
    }
}

impl Sub<&M31> for M31 {
    type Output = M31;
    #[inline(always)]
    fn sub(self, rhs: &M31) -> Self::Output {
        self + &(-*rhs)
    }
}

impl Sub for M31 {
    type Output = M31;
    #[inline(always)]
    fn sub(self, rhs: M31) -> Self::Output {
        self - &rhs
    }
}

impl AddAssign<&M31> for M31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &M31) {
        *self = *self + rhs;
    }
}

impl AddAssign for M31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl From<u32> for M31 {
    #[inline(always)]
    fn from(x: u32) -> Self {
        M31 {
            v: if x < M31_MOD as u32 {
                x
            } else {
                x % M31_MOD as u32
            },
        }
    }
}

#[cfg(target_arch = "x86_64")]
pub mod m31_avx;
#[cfg(target_arch = "x86_64")]
pub use m31_avx::{PackedM31, M31_PACK_SIZE, M31_VECTORIZE_SIZE};

#[cfg(target_arch = "aarch64")]
pub mod m31_neon;
#[cfg(target_arch = "aarch64")]
pub use m31_neon::{PackedM31, M31_PACK_SIZE, M31_VECTORIZE_SIZE};

#[cfg(target_arch = "x86_64")]
use self::m31_avx::PACKED_INV_2;
#[cfg(target_arch = "aarch64")]
use self::m31_neon::PACKED_INV_2;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VectorizedM31 {
    pub v: [PackedM31; M31_VECTORIZE_SIZE],
}

pub const VECTORIZEDM31_INV_2: VectorizedM31 = VectorizedM31 {
    v: [PackedM31 { v: PACKED_INV_2 }; M31_VECTORIZE_SIZE],
};

impl VectorizedM31 {
    pub const SIZE: usize = size_of::<[PackedM31; M31_VECTORIZE_SIZE]>();
    #[inline(always)]
    pub fn serialize_into(&self, buffer: &mut [u8]) {
        buffer.copy_from_slice(unsafe {
            std::slice::from_raw_parts(
                self.v.as_ptr() as *const u8,
                M31_VECTORIZE_SIZE * PackedM31::SIZE,
            )
        });
    }
    #[inline(always)]
    pub fn deserialize_from(buffer: &[u8]) -> Self {
        let ptr = buffer.as_ptr() as *const [PackedM31; M31_VECTORIZE_SIZE];
        unsafe {
            VectorizedM31 {
                v: ptr.read_unaligned(),
            }
        }
    }
}

impl Field for VectorizedM31 {
    #[inline(always)]
    fn zero() -> Self {
        VectorizedM31 {
            v: [PackedM31::zero(); M31_VECTORIZE_SIZE],
        }
    }

    #[inline(always)]
    fn one() -> Self {
        VectorizedM31 {
            v: [PackedM31::one(); M31_VECTORIZE_SIZE],
        }
    }

    #[inline(always)]
    fn random() -> Self {
        VectorizedM31 {
            v: (0..M31_VECTORIZE_SIZE)
                .map(|_| PackedM31::random())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }

    #[inline(always)]
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
    #[inline(always)]
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
    #[inline(always)]
    fn mul(self, rhs: VectorizedM31) -> Self::Output {
        self * &rhs
    }
}

impl Mul<&M31> for VectorizedM31 {
    type Output = VectorizedM31;
    #[inline(always)]
    fn mul(self, rhs: &M31) -> Self::Output {
        let mut v = [PackedM31::zero(); M31_VECTORIZE_SIZE];
        let packed_rhs = PackedM31::pack_full(*rhs);
        for i in 0..M31_VECTORIZE_SIZE {
            v[i] = self.v[i] * packed_rhs;
        }
        VectorizedM31 { v }
    }
}

impl Mul<M31> for VectorizedM31 {
    type Output = VectorizedM31;
    #[inline(always)]
    fn mul(self, rhs: M31) -> Self::Output {
        self * &rhs
    }
}

impl Add<&VectorizedM31> for VectorizedM31 {
    type Output = VectorizedM31;
    #[inline(always)]
    fn add(self, rhs: &VectorizedM31) -> Self::Output {
        VectorizedM31 {
            v: self
                .v
                .iter()
                .zip(rhs.v.iter())
                .map(|(a, b)| *a + b)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }
}

impl Add for VectorizedM31 {
    type Output = VectorizedM31;
    #[inline(always)]
    fn add(self, rhs: VectorizedM31) -> Self::Output {
        self + &rhs
    }
}

impl AddAssign<&VectorizedM31> for VectorizedM31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &VectorizedM31) {
        self.v
            .iter_mut()
            .zip(rhs.v.iter())
            .for_each(|(a, b)| *a += b);
    }
}

impl AddAssign for VectorizedM31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl AddAssign<&M31> for VectorizedM31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &M31) {
        self.v
            .iter_mut()
            .for_each(|x| *x += PackedM31::pack_full(*rhs));
    }
}

impl AddAssign<M31> for VectorizedM31 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: M31) {
        *self += &rhs;
    }
}

impl Sub for VectorizedM31 {
    type Output = VectorizedM31;
    #[inline(always)]
    fn sub(self, rhs: VectorizedM31) -> Self::Output {
        VectorizedM31 {
            v: self
                .v
                .iter()
                .zip(rhs.v.iter())
                .map(|(a, b)| *a - b)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }
}

impl From<u32> for VectorizedM31 {
    #[inline(always)]
    fn from(x: u32) -> Self {
        VectorizedM31 {
            v: [PackedM31::from(x); M31_VECTORIZE_SIZE],
        }
    }
}
