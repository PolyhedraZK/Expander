mod m31;
pub use m31::*;
mod bn254;
pub use bn254::*;

use rand::RngCore;

use std::{
    fmt::Debug,
    iter::{Product, Sum},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

/// Field definitions.
pub trait Field:
    Copy
    + Clone
    + Debug
    + Default
    + PartialEq
    + From<u32>
    + Neg<Output = Self>
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Sum
    + Product
    + for<'a> Add<&'a Self, Output = Self>
    + for<'a> Sub<&'a Self, Output = Self>
    + for<'a> Mul<&'a Self, Output = Self>
    + for<'a> Sum<&'a Self>
    + for<'a> Product<&'a Self>
    + AddAssign
    + SubAssign
    + MulAssign
    + for<'a> AddAssign<&'a Self>
    + for<'a> SubAssign<&'a Self>
    + for<'a> MulAssign<&'a Self>
{
    /// name
    const NAME: &'static str;

    /// size required to store the data
    const SIZE: usize;

    /// Inverse of 2
    const INV_2: Self;

    /// type of the base field, can be itself
    type BaseField: Field + FieldSerde;

    // ====================================
    // constants
    // ====================================
    /// Zero element
    fn zero() -> Self;

    /// Is zero
    fn is_zero(&self) -> bool {
        *self == Self::zero()
    }

    /// Identity element
    fn one() -> Self;

    // ====================================
    // generators
    // ====================================
    /// create a random element from rng.
    /// test only -- the output may not be uniformly random.
    fn random_unsafe(rng: impl RngCore) -> Self;

    /// create a random boolean element from rng
    fn random_bool_unsafe(rng: impl RngCore) -> Self;

    // ====================================
    // arithmetics
    // ====================================
    /// Squaring
    fn square(&self) -> Self {
        *self * *self
    }

    /// Doubling
    fn double(&self) -> Self {
        *self + *self
    }

    /// Exp
    fn exp(&self, exponent: &Self) -> Self;

    /// find the inverse of the element; return None if not exist
    fn inv(&self) -> Option<Self>;

    /// Add the field element with its base field element
    fn add_base_elem(&self, rhs: &Self::BaseField) -> Self;

    /// Add the field element with its base field element
    fn add_assign_base_elem(&mut self, rhs: &Self::BaseField);

    /// multiply the field element with its base field element
    fn mul_base_elem(&self, rhs: &Self::BaseField) -> Self;

    /// multiply the field element with its base field element
    fn mul_assign_base_elem(&mut self, rhs: &Self::BaseField);

    /// expose the element as u32.
    fn as_u32_unchecked(&self) -> u32;

    /// sample from a 32 bytes
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self;
}

/// A vector of Field elements.
pub trait VectorizedField: Field {
    /// type of the packed based field, if applicable
    type PackedBaseField: Default + Clone;

    /// expose the internal elements
    fn as_packed_slices(&self) -> &[Self::PackedBaseField];

    /// expose the internal elements mutable
    fn mut_packed_slices(&mut self) -> &mut [Self::PackedBaseField];
}

/// Serde for Fields
pub trait FieldSerde {
    /// serialize self into bytes
    fn serialize_into(&self, buffer: &mut [u8]);

    /// deserialize bytes into field
    fn deserialize_from(buffer: &[u8]) -> Self;
}
