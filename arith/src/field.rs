mod m31;
pub use m31::*;
mod vectorized_m31;
pub use vectorized_m31::*;

use std::{
    fmt::Debug,
    iter::{Product, Sum},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

// TODO: we may want to enrich this trait definition, and allow for more complicated derivations, such as Serde.
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

    /// type of the base field, can be itself
    type BaseField: Field + FieldSerde;

    /// Zero element
    fn zero() -> Self;

    /// Identity element
    fn one() -> Self;

    /// create a random element from rng
    fn random() -> Self;

    /// create a random boolean element from rng
    fn random_bool() -> Self;

    /// find the inverse of the element
    fn inv(&self) -> Self;

    /// multiply the field element with its base field element
    fn mul_by_base(&self, rhs: &Self::BaseField) -> Self;

    /// expose the element as u32.
    fn as_u32_unchecked(&self) -> u32;
}

pub trait FieldSerde {
    /// serialize self into bytes
    fn serialize_into(&self, buffer: &mut [u8]);

    /// deserialize bytes into field
    fn deserialize_from(buffer: &[u8]) -> Self;
}
