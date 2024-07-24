use arith::{Field, FieldSerde, SimdField};

use crate::{Proof, SHA256hasher};

pub struct Transcript {
    pub hasher: SHA256hasher,
    hash_start_idx: usize,
    digest: [u8; Self::DIGEST_SIZE],
    pub proof: Proof,
}

impl Default for Transcript {
    fn default() -> Self {
        Self::new()
    }
}

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

    #[inline]
    pub fn new() -> Self {
        Transcript {
            hasher: SHA256hasher,
            hash_start_idx: 0,
            digest: [0u8; Self::DIGEST_SIZE],
            proof: Proof::default(),
        }
    }
    #[inline]
    pub fn append_f<F: Field + FieldSerde>(&mut self, f: F) {
        let cur_size = self.proof.bytes.len();
        self.proof.bytes.resize(cur_size + F::SIZE, 0);
        f.serialize_into(&mut self.proof.bytes[cur_size..]);
    }
    #[inline]
    pub fn append_u8_slice(&mut self, buffer: &[u8]) {
        self.proof.bytes.extend_from_slice(buffer);
    }
    #[inline]
    pub fn challenge_f<F: SimdField>(&mut self) -> F::Scalar {
        self.hash_to_digest();
        assert!(F::Scalar::SIZE <= Self::DIGEST_SIZE);
        F::Scalar::from_uniform_bytes(&self.digest)
    }
    #[inline]
    pub fn challenge_fs<F: SimdField>(&mut self, size: usize) -> Vec<F::Scalar> {
        (0..size).map(|_| self.challenge_f::<F>()).collect()
    }
}
