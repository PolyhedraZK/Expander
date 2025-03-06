use tiny_keccak::{Hasher, Keccak};

use super::FiatShamirBytesHash;

#[derive(Clone, Default, Debug)]
pub struct Keccak256hasher {}

impl FiatShamirBytesHash for Keccak256hasher {
    const DIGEST_SIZE: usize = 32;

    #[inline]
    fn new() -> Keccak256hasher {
        Keccak256hasher {}
    }

    #[inline]
    fn hash(output: &mut [u8], input: &[u8]) {
        let mut hasher = Keccak::v256();
        hasher.update(input);
        hasher.finalize(output);
    }

    #[inline]
    fn hash_inplace(buffer: &mut [u8]) {
        let mut hasher = Keccak::v256();
        hasher.update(&*buffer);
        hasher.finalize(buffer);
    }
}
