use tiny_keccak::{Hasher, Sha3};

use super::FiatShamirBytesHash;

#[derive(Clone, Default)]
pub struct Keccak256hasher {}

impl FiatShamirBytesHash for Keccak256hasher {
    const DIGEST_SIZE: usize = 32;

    #[inline]
    fn new() -> Keccak256hasher {
        Keccak256hasher {}
    }

    #[inline]
    fn hash(output: &mut [u8], input: &[u8]) {
        let mut hasher = Sha3::v256();
        hasher.update(input);
        hasher.finalize(output);
    }

    #[inline]
    fn hash_inplace(buffer: &mut [u8]) {
        let mut hasher = Sha3::v256();
        hasher.update(&*buffer);
        hasher.finalize(buffer);
    }
}
