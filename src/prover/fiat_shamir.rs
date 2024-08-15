use arith::{Field, FieldSerde};

use crate::{GKRConfig, Proof, SHA256hasher};

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

    #[inline]
    fn hash_to_digest(&mut self) {
        let hash_end_idx = self.proof.bytes.len();
        if hash_end_idx > self.hash_start_idx {
            self.hasher.hash(
                &mut self.digest,
                &self.proof.bytes[self.hash_start_idx..hash_end_idx],
            );
            self.hash_start_idx = hash_end_idx;
        } else {
            self.hasher.hash_inplace(&mut self.digest)
        }
    }

    #[inline]
    pub fn new() -> Self {
        Transcript {
            hasher: SHA256hasher::new(),
            hash_start_idx: 0,
            digest: [0u8; Self::DIGEST_SIZE],
            proof: Proof::default(),
        }
    }

    #[inline]
    pub fn append_f<C: GKRConfig>(&mut self, f: C::Field) {
        let cur_size = self.proof.bytes.len();
        println!("cur_size: {}", cur_size);
        println!("C::Field::SIZE: {}", C::Field::SIZE);
        self.proof.bytes.resize(cur_size + C::Field::SIZE, 0);
        f.serialize_into(&mut self.proof.bytes[cur_size..]);
    }

    #[inline]
    pub fn append_u8_slice(&mut self, buffer: &[u8]) {
        self.proof.bytes.extend_from_slice(buffer);
    }

    #[inline]
    pub fn challenge_f<C: GKRConfig>(&mut self) -> C::ChallengeField {
        self.hash_to_digest();
        assert!(C::ChallengeField::SIZE <= Self::DIGEST_SIZE);
        C::ChallengeField::from_uniform_bytes(&self.digest)
    }

    #[inline]
    pub fn challenge_fs<C: GKRConfig>(&mut self, size: usize) -> Vec<C::ChallengeField> {
        (0..size).map(|_| self.challenge_f::<C>()).collect()
    }

    #[inline]
    pub fn circuit_f<C: GKRConfig>(&mut self) -> C::CircuitField {
        self.hash_to_digest();
        assert!(C::CircuitField::SIZE <= Self::DIGEST_SIZE);
        C::CircuitField::from_uniform_bytes(&self.digest)
    }
}
