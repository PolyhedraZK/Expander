use arith::{Field, FieldSerde};

pub mod sha2_256;
pub use sha2_256::*;

pub mod keccak_256;
pub use keccak_256::*;

pub mod mimc;
pub use mimc::*;

pub trait FiatShamirBytesHash {
    /// The size of the hash output in bytes.
    const DIGEST_SIZE: usize;

    /// Create a new hash instance.
    fn new() -> Self;

    /// Hash the input into the output.
    fn hash(output: &mut [u8], input: &[u8]);

    /// Hash the input in place.
    fn hash_inplace(buffer: &mut [u8]);
}

pub trait FiatShamirFieldHash<F: Field + FieldSerde> {
    /// Create a new hash instance.
    fn new() -> Self;

    /// hash a vector of field element and return the hash result
    fn hash(&self, input: &[F]) -> F;
}
