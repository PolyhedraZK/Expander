use std::{fmt::Debug, marker::PhantomData};

use arith::{ExtensionField, Field, FieldForECC};

use crate::{
    fiat_shamir_hash::{FiatShamirBytesHash, FiatShamirFieldHash},
    Proof,
};

pub trait Transcript<F: Field>: Clone + Debug {
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

    /// Generate a list of positions that we want to open the polynomial at.
    #[inline]
    fn generate_challenge_index_vector(&mut self, num_queries: usize) -> Vec<usize> {
        let mut challenges = Vec::with_capacity(num_queries);
        let mut buf = [0u8; 8];
        for _ in 0..num_queries {
            buf.copy_from_slice(self.generate_challenge_u8_slice(8).as_slice());
            challenges.push(usize::from_le_bytes(buf));
        }
        challenges
    }

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

    /// Return current state of the transcript
    /// Note: this may incur an additional hash to shrink the state
    fn hash_and_return_state(&mut self) -> Vec<u8>;

    /// Set the state
    /// Note: Any unhashed data will be discarded
    fn set_state(&mut self, state: &[u8]);

    /// lock proof, no changes will be made to the proof until unlock is called
    fn lock_proof(&mut self);

    /// unlock proof
    fn unlock_proof(&mut self);
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct BytesHashTranscript<F: Field, H: FiatShamirBytesHash> {
    phantom: PhantomData<(F, H)>,

    /// The digest bytes.
    pub digest: Vec<u8>,

    /// The proof bytes.
    proof: Proof,

    /// The pointer to the proof bytes indicating where the hash starts.
    hash_start_index: usize,

    /// locking point
    proof_locked: bool,
    proof_locked_at: usize,
}

impl<F: Field, H: FiatShamirBytesHash> Transcript<F> for BytesHashTranscript<F, H> {
    fn new() -> Self {
        Self {
            phantom: PhantomData,
            digest: vec![0u8; H::DIGEST_SIZE],
            proof: Proof::default(),
            hash_start_index: 0,
            proof_locked: false,
            proof_locked_at: 0,
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

    fn hash_and_return_state(&mut self) -> Vec<u8> {
        self.hash_to_digest();
        self.digest.clone()
    }

    fn set_state(&mut self, state: &[u8]) {
        self.hash_start_index = self.proof.bytes.len(); // discard unhashed data
        assert!(state.len() == H::DIGEST_SIZE);
        self.digest = state.to_vec();
    }

    fn lock_proof(&mut self) {
        assert!(!self.proof_locked);
        self.proof_locked = true;
        self.proof_locked_at = self.proof.bytes.len();
    }

    fn unlock_proof(&mut self) {
        assert!(self.proof_locked);
        self.proof_locked = false;
        if self.hash_start_index < self.proof.bytes.len() {
            self.hash_to_digest();
        }
        self.proof.bytes.resize(self.proof_locked_at, 0);
        self.hash_start_index = self.proof.bytes.len();
    }
}

impl<F: Field, H: FiatShamirBytesHash> BytesHashTranscript<F, H> {
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

#[derive(Clone, Debug, PartialEq)]
pub struct FieldHashTranscript<
    F: FieldForECC,
    ExtF: ExtensionField<BaseField = F>,
    H: FiatShamirFieldHash<F, ExtF>,
> {
    /// Internal hasher, it's a little costly to create a new hasher
    pub hasher: H,

    /// The digest bytes.
    pub digest: ExtF,

    /// The proof bytes
    pub proof: Proof,

    /// The data to be hashed
    pub data_pool: Vec<ExtF>,

    /// Proof locked or not
    pub proof_locked: bool,
}

impl<F: FieldForECC, ExtF: ExtensionField<BaseField = F>, H: FiatShamirFieldHash<F, ExtF>>
    Transcript<ExtF> for FieldHashTranscript<F, ExtF, H>
{
    #[inline(always)]
    fn new() -> Self {
        Self {
            hasher: H::new(),
            digest: ExtF::default(),
            proof: Proof::default(),
            data_pool: vec![],
            proof_locked: false,
        }
    }

    fn append_field_element(&mut self, f: &ExtF) {
        let mut buffer = vec![];
        f.serialize_into(&mut buffer).unwrap();
        if !self.proof_locked {
            self.proof.bytes.extend_from_slice(&buffer);
        }
        self.data_pool.push(*f);
    }

    fn append_u8_slice(&mut self, buffer: &[u8]) {
        if !self.proof_locked {
            self.proof.bytes.extend_from_slice(buffer);
        }
        let buffer_size = buffer.len();
        let mut cur = 0;
        while cur + 32 <= buffer_size {
            self.data_pool.push(ExtF::from_uniform_bytes(
                buffer[cur..cur + 32].try_into().unwrap(),
            ));
            cur += 32
        }

        if cur < buffer_size {
            let mut buffer_last = buffer[cur..].to_vec();
            buffer_last.resize(32, 0);
            self.data_pool.push(ExtF::from_uniform_bytes(
                buffer_last[..].try_into().unwrap(),
            ));
        }
    }

    fn generate_challenge_field_element(&mut self) -> ExtF {
        self.hash_to_digest();
        self.digest
    }

    fn generate_challenge_u8_slice(&mut self, n_bytes: usize) -> Vec<u8> {
        let mut bytes = vec![];
        let mut buf = vec![];
        while bytes.len() < n_bytes {
            self.hash_to_digest();
            self.digest.serialize_into(&mut buf).unwrap();
            bytes.extend_from_slice(&buf);
        }
        bytes.resize(n_bytes, 0);
        bytes
    }

    fn finalize_and_get_proof(&self) -> Proof {
        self.proof.clone()
    }

    fn hash_and_return_state(&mut self) -> Vec<u8> {
        self.hash_to_digest();
        let mut state = vec![];
        self.digest.serialize_into(&mut state).unwrap();
        state
    }

    fn set_state(&mut self, state: &[u8]) {
        self.data_pool.clear();
        self.digest = ExtF::deserialize_from(state).unwrap();
    }

    fn lock_proof(&mut self) {
        assert!(!self.proof_locked);
        self.proof_locked = true;
    }

    fn unlock_proof(&mut self) {
        assert!(self.proof_locked);
        self.proof_locked = false;
    }
}

impl<F: FieldForECC, ExtF: ExtensionField<BaseField = F>, H: FiatShamirFieldHash<F, ExtF>>
    FieldHashTranscript<F, ExtF, H>
{
    pub fn hash_to_digest(&mut self) {
        if !self.data_pool.is_empty() {
            self.digest = self.hasher.hash(&self.data_pool);
            self.data_pool.clear();
        } else {
            self.digest = self.hasher.hash(&[self.digest]);
        }
    }
}
