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
    + From<usize>
    + Mul<Output = Self>
    + for<'a> Mul<&'a Self, Output = Self>
    + AddAssign
    + for<'a> AddAssign<&'a Self>
{
    fn zero() -> Self;
    fn one() -> Self;
    fn random() -> Self;
    fn random_bool() -> Self;
    fn inv(&self) -> Self;
}

pub mod m31;

pub use m31::*;

pub mod poly;
pub use poly::*;
