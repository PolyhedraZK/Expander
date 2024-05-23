use arith::{Field, FieldSerde, VectorizedM31, M31};

use crate::{Proof, SHA256hasher};

pub struct Transcript {
    pub hasher: SHA256hasher,
    hash_start_idx: usize,
    digest: [u8; Self::DIGEST_SIZE],
    pub proof: Proof,
}

type FPrimitive = M31;
type F = VectorizedM31;

impl Transcript {
    pub const DIGEST_SIZE: usize = 32;

    fn hash_to_digest(&mut self) {
        let hash_end_idx = self.proof.bytes.len();
        if hash_end_idx > self.hash_start_idx {
            self.hasher.hash(
                &mut self.digest,
                &self.proof.bytes[self.hash_start_idx..],
                hash_end_idx - self.hash_start_idx,
            );
            self.hash_start_idx = hash_end_idx;
        } else {
            self.hasher
                .hash_inplace(&mut self.digest, Self::DIGEST_SIZE)
        }
    }

    pub fn new() -> Self {
        Transcript {
            hasher: SHA256hasher::default(),
            hash_start_idx: 0,
            digest: [0u8; Self::DIGEST_SIZE],
            proof: Proof::default(),
        }
    }

    pub fn append_f(&mut self, f: F) {
        let cur_size = self.proof.bytes.len();
        self.proof.bytes.resize(cur_size + F::SIZE, 0);
        f.serialize_into(&mut self.proof.bytes[cur_size..]);
    }

    pub fn append_u8_slice(&mut self, buffer: &[u8], size: usize) {
        self.proof.append_u8_slice(buffer, size);
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
