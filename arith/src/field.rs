mod m31;
pub use m31::*;

use std::{
    fmt::Debug,
    ops::{AddAssign, Mul},
};

pub trait Field:
    Copy
    + Clone
    + Debug
    + Default
    + PartialEq
    + From<u32>
    + Mul<Output = Self>
    + for<'a> Mul<&'a Self, Output = Self>
    + AddAssign
    + for<'a> AddAssign<&'a Self>
{
    /// name
    const NAME: &'static str;

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
}
