use std::fmt::Debug;

use arith::ExtensionField;

pub mod sha2_256;
pub use sha2_256::*;

pub mod keccak_256;
pub use keccak_256::*;

pub mod mimc;
pub use mimc::*;

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

// TODO(HS) actually we should change this to FiatShamirSponge
// TODO(HS) should hash to some hash state rather than extf
pub trait FiatShamirFieldHash<ExtF: ExtensionField>: Clone + Debug + Default {
    /// Create a new hash instance.
    fn new() -> Self;

    /// hash a vector of field element and return the hash result
    fn hash(&self, input: &[ExtF]) -> ExtF;
}
