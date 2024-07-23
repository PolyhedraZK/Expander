mod bn254;
mod m31;
mod m31_ext;

pub use m31::*;
pub use m31_ext::*;

use rand::RngCore;

use std::{
    fmt::Debug,
    io::{Read, Write},
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
    fn random_bool(rng: impl RngCore) -> Self;

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

    /// expose the element as u32.
    fn as_u32_unchecked(&self) -> u32;

    /// sample from a 32 bytes
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self;
}

/// Configurations for the Fiat-Shamir transform.
pub trait FiatShamirConfig: From<Self::ChallengeField> {
    // todo: consolidate gkr config

    /// Field for the challenge. Can be self.
    type ChallengeField: Field + FieldSerde + Send;

    /// scale self with the challenge
    fn scale(&self, challenge: &Self::ChallengeField) -> Self;
}

/// Serde for Fields
pub trait FieldSerde {
    /// serialize self into bytes
    fn serialize_into<W: Write>(&self, writer: W);

    /// size of the serialized bytes
    fn serialized_size() -> usize;

    /// deserialize bytes into field
    fn deserialize_from<R: Read>(reader: R) -> Self;

    /// deserialize bytes into field following ecc format
    fn deserialize_from_ecc_format<R: Read>(_reader: R) -> Self;
}

impl FieldSerde for u64 {
    /// serialize u64 into bytes
    fn serialize_into<W: Write>(&self, mut writer: W) {
        writer.write_all(&self.to_le_bytes()).unwrap();
    }

    /// size of the serialized bytes
    fn serialized_size() -> usize {
        8
    }

    /// deserialize bytes into u64
    fn deserialize_from<R: Read>(mut reader: R) -> Self {
        let mut buffer = [0u8; 8];
        reader.read_exact(&mut buffer).unwrap();
        u64::from_le_bytes(buffer)
    }

    fn deserialize_from_ecc_format<R: Read>(_reader: R) -> Self {
        unimplemented!("not implemented for u64")
    }
}
