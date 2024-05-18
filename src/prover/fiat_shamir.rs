use crate::{Proof, SHA256hasher, VectorizedM31, M31};

pub struct Transcript {
    pub hasher: SHA256hasher,
    digest: [u8; Self::DIGEST_SIZE],
    pub proof: Proof,
}

type FPrimitive = M31;
type F = VectorizedM31;

impl Transcript {
    pub const DIGEST_SIZE: usize = 32;

    fn hash_to_digest(&mut self) {
        todo!()
    }

    pub fn new() -> Self {
        todo!()
    }

    pub fn append_u8_slice(&self, buffer: &[u8], size: usize) {
        todo!()
    }

    pub fn challenge_f(&mut self) -> FPrimitive {
        self.hash_to_digest();
        assert!(FPrimitive::SIZE <= Self::DIGEST_SIZE);
        FPrimitive::deserialize_from(&self.digest)
    }

    pub fn challenge_fs(&mut self, size: usize) -> Vec<FPrimitive> {
        (0..size).map(|_| self.challenge_f()).collect()
    }
}
