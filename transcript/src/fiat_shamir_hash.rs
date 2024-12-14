use std::fmt::Debug;

use arith::{ExtensionField, FieldForECC};

pub mod sha2_256;
pub use sha2_256::*;

pub mod keccak_256;
pub use keccak_256::*;

pub mod mimc;
pub use mimc::*;

pub mod poseidon;
#[allow(unused)]
pub use poseidon::*;

pub trait FiatShamirBytesHash: Clone + Debug {
    /// The size of the hash output in bytes.
    const DIGEST_SIZE: usize;

    /// Create a new hash instance.
    fn new() -> Self;

    /// Hash the input into the output.
    fn hash(output: &mut [u8], input: &[u8]);

    /// Hash the input in place.
    fn hash_inplace(buffer: &mut [u8]);
}

pub trait FiatShamirFieldHash<F: FieldForECC, ExtF: ExtensionField<BaseField = F>>:
    Clone + Debug
{
    /// Create a new hash instance.
    fn new() -> Self;

    /// hash a vector of field element and return the hash result
    fn hash(&self, input: &[ExtF]) -> ExtF;
}
