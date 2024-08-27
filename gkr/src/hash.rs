pub mod sha2_256;
pub use self::sha2_256::*;

pub mod keccak_256;
pub use self::keccak_256::*;

pub trait FiatShamirHash {
    const DIGEST_SIZE: usize;

    fn new() -> Self;

    fn hash(&mut self, output: &mut [u8], input: &[u8]);

    fn hash_inplace(&mut self, buffer: &mut [u8]);
}
