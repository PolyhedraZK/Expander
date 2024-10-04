use std::marker::PhantomData;

use arith::{Field, FieldSerde};

use crate::{fiat_shamir_hash::{FiatShamirBytesHash, FiatShamirFieldHash}, Proof};

pub trait Transcript<F: Field + FieldSerde> {
    /// Create a new transcript.
    fn new() -> Self;

    /// Append a field element to the transcript.
    fn append_field_element(&mut self, f: &F);
    
    /// Append a slice of bytes
    fn append_u8_slice(&mut self, buffer: &[u8]);

    /// Generate a challenge.
    fn generate_challenge_field_element(&mut self) -> F;

    /// Generate a slice of random bytes of some fixed size
    /// Use this function when you need some randomness other than the native field
    fn generate_challenge_u8_slice(&mut self, n_bytes: usize) -> Vec<u8>;

    /// Generate a challenge vector.
    #[inline]
    fn generate_challenge_field_elements(&mut self, n: usize) -> Vec<F> {
        let mut challenges = Vec::with_capacity(n);
        for _ in 0..n {
            challenges.push(self.generate_challenge_field_element());
        }
        challenges
    }

    /// Produce the proof
    /// It is not recommended to append/challenge after calling this function
    fn finalize_and_get_proof(&self) -> Proof;
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct BytesHashTranscript<H: FiatShamirBytesHash> {
    phantom: PhantomData<H>,

    /// The digest bytes.
    pub digest: Vec<u8>,

    /// The proof bytes.
    pub proof: Proof,

    /// The pointer to the proof bytes indicating where the hash starts.
    hash_start_index: usize,
}

impl<F: Field + FieldSerde, H: FiatShamirBytesHash> Transcript<F> for BytesHashTranscript<H> {
    fn new() -> Self {
        Self {
            phantom: PhantomData,
            digest: vec![0u8; H::DIGEST_SIZE],
            proof: Proof::default(),
            hash_start_index: 0,
        }
    }

    fn append_field_element(&mut self, f: &F) {
        let mut buf = vec![];
        f.serialize_into(&mut buf).unwrap();
        self.append_u8_slice(&buf);
    }

    /// Append a byte slice to the transcript.
    fn append_u8_slice(&mut self, buffer: &[u8]) {
        self.proof.bytes.extend_from_slice(buffer);
    }

    /// Generate a challenge.
    fn generate_challenge_field_element(&mut self) -> F {
        self.hash_to_digest();
        assert!(F::SIZE <= H::DIGEST_SIZE);
        F::from_uniform_bytes(&self.digest.clone().try_into().unwrap())
    }

    fn generate_challenge_u8_slice(&mut self, n_bytes: usize) -> Vec<u8> {
        let mut ret = vec![];
        let mut cur_n_bytes = 0usize;

        while cur_n_bytes < n_bytes {
            self.hash_to_digest();
            ret.extend_from_slice(&self.digest);
            cur_n_bytes += H::DIGEST_SIZE;
        }

        ret.resize(n_bytes, 0);
        ret
    }

    fn finalize_and_get_proof(&self) -> Proof {
        self.proof.clone()
    }

}

impl<H: FiatShamirBytesHash> BytesHashTranscript<H> {
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

#[derive(Clone, Default, Debug, PartialEq)]
pub struct FieldHashTranscript<F: Field+FieldSerde, H: FiatShamirFieldHash<F>> {
    phantom: PhantomData<(F, H)>,

    /// The digest bytes.
    pub digest: Vec<u8>,

    /// The proof bytes.
    pub proof: Proof,

    /// The pointer to the proof bytes indicating where the hash starts.
    hash_start_index: usize,
}

impl <F: Field+FieldSerde, H: FiatShamirFieldHash<F>> Transcript for FieldHashTranscript<F, H> {
    fn new() -> Self {
        todo!()
    }

    fn append_field_element(&mut self, f: &F) {
        todo!()
    }

    fn append_u8_slice(&mut self, buffer: &[u8]) {
        todo!()
    }

    fn generate_challenge(&mut self) -> F {
        todo!()
    }

    fn finalize_and_get_proof(&self) -> Proof {
        todo!()
    }
}