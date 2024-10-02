use std::marker::PhantomData;

use arith::{Field, FieldSerde};

use crate::{fiat_shamir_hash::FiatShamirHash, Proof};

pub trait Transcript<H: FiatShamirHash> {
    /// Create a new transcript.
    fn new() -> Self;

    /// Append a field element to the transcript.
    fn append_field_element<F>(&mut self, f: &F)
    where
        F: FieldSerde,
    {
        let mut buf = vec![];
        f.serialize_into(&mut buf).unwrap();
        self.append_u8_slice(&buf);
    }

    /// Append a byte slice to the transcript.
    fn append_u8_slice(&mut self, buffer: &[u8]);

    /// Generate a challenge.
    fn generate_challenge<F: Field>(&mut self) -> F;

    /// Generate a challenge vector.
    #[inline]
    fn generate_challenge_vector<F: Field>(&mut self, n: usize) -> Vec<F> {
        let mut challenges = Vec::with_capacity(n);
        for _ in 0..n {
            challenges.push(self.generate_challenge());
        }
        challenges
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct TranscriptInstance<H: FiatShamirHash> {
    phantom: PhantomData<H>,

    /// The digest bytes.
    pub digest: Vec<u8>,

    /// The proof bytes.
    pub proof: Proof,

    /// The pointer to the proof bytes indicating where the hash starts.
    hash_start_index: usize,
}

impl<H: FiatShamirHash> Transcript<H> for TranscriptInstance<H> {
    fn new() -> Self {
        TranscriptInstance {
            phantom: PhantomData,
            digest: vec![0u8; H::DIGEST_SIZE],
            proof: Proof::default(),
            hash_start_index: 0,
        }
    }

    /// Append a byte slice to the transcript.
    fn append_u8_slice(&mut self, buffer: &[u8]) {
        self.proof.bytes.extend_from_slice(buffer);
    }

    /// Generate a challenge.
    fn generate_challenge<F: Field>(&mut self) -> F {
        self.hash_to_digest();
        assert!(F::SIZE <= H::DIGEST_SIZE);
        F::from_uniform_bytes(&self.digest.clone().try_into().unwrap())
    }
}

impl<H: FiatShamirHash> TranscriptInstance<H> {
    /// Hash the input into the output.
    pub fn hash_to_digest(&mut self) {
        let hash_end_index = self.proof.bytes.len();
        if hash_end_index > self.hash_start_index {
            H::hash(
                &mut self.digest,
                &self.proof.bytes[self.hash_start_index..hash_end_index],
            );
            self.hash_start_index = hash_end_index;
        } else {
            H::hash_inplace(&mut self.digest);
        }
    }
}
