use std::{fmt::Debug, marker::PhantomData};

use arith::{ExtensionField, Field};
use hasher::{FiatShamirSponge, FieldHasherState};

use crate::{fiat_shamir_hash::FiatShamirBytesHash, Proof};

pub trait Transcript<BaseF: Field, ChallengeF: ExtensionField<BaseField = BaseF>>:
    Clone + Debug
{
    /// Create a new transcript.
    fn new() -> Self;

    /// Append a field element to the transcript.
    fn append_field_element(&mut self, f: &ChallengeF);

    /// Append a slice of bytes
    fn append_u8_slice(&mut self, buffer: &[u8]);

    /// Generate a challenge.
    fn generate_challenge_field_element(&mut self) -> ChallengeF;

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
    fn generate_challenge_field_elements(&mut self, n: usize) -> Vec<ChallengeF> {
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

impl<BaseF: Field, ChallengeF: ExtensionField<BaseField = BaseF>, H: FiatShamirBytesHash>
    Transcript<BaseF, ChallengeF> for BytesHashTranscript<ChallengeF, H>
{
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

    fn append_field_element(&mut self, f: &ChallengeF) {
        let mut buf = vec![];
        f.serialize_into(&mut buf).unwrap();
        self.append_u8_slice(&buf);
    }

    /// Append a byte slice to the transcript.
    fn append_u8_slice(&mut self, buffer: &[u8]) {
        self.proof.bytes.extend_from_slice(buffer);
    }

    /// Generate a challenge.
    fn generate_challenge_field_element(&mut self) -> ChallengeF {
        self.hash_to_digest();
        assert!(ChallengeF::SIZE <= H::DIGEST_SIZE);
        ChallengeF::from_uniform_bytes(&self.digest.clone().try_into().unwrap())
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

#[derive(Default, Clone, Debug, PartialEq)]
pub struct FieldHashTranscript<BaseF, ChallengeF, State, Sponge>
where
    BaseF: Field,
    ChallengeF: ExtensionField<BaseField = BaseF>,
    State: FieldHasherState<InputF = BaseF, OutputF = ChallengeF>,
    Sponge: FiatShamirSponge<State>,
{
    /// Internal hasher, it's a little costly to create a new hasher
    pub fs_sponge: Sponge,

    /// The proof bytes
    pub proof: Proof,

    /// Proof locked or not
    pub proof_locked: bool,

    _phantom: PhantomData<State>,
}

impl<BaseF, ChallengeF, State, Sponge> Transcript<BaseF, ChallengeF>
    for FieldHashTranscript<BaseF, ChallengeF, State, Sponge>
where
    BaseF: Field,
    ChallengeF: ExtensionField<BaseField = BaseF>,
    State: FieldHasherState<InputF = BaseF, OutputF = ChallengeF>,
    Sponge: FiatShamirSponge<State>,
{
    #[inline(always)]
    fn new() -> Self {
        Self {
            fs_sponge: Sponge::new(),
            ..Default::default()
        }
    }

    fn append_field_element(&mut self, f: &ChallengeF) {
        let mut buffer = vec![];
        f.serialize_into(&mut buffer).unwrap();
        if !self.proof_locked {
            self.proof.bytes.extend_from_slice(&buffer);
        }
        self.fs_sponge.update(&f.to_limbs());
    }

    fn append_u8_slice(&mut self, buffer: &[u8]) {
        if !self.proof_locked {
            self.proof.bytes.extend_from_slice(buffer);
        }
        let mut buffer_local = buffer.to_vec();
        buffer_local.resize(buffer_local.len().next_multiple_of(32), 0u8);
        buffer_local.chunks_exact(32).for_each(|chunk_32| {
            let c = ChallengeF::from_uniform_bytes(chunk_32.try_into().unwrap());
            self.fs_sponge.update(&c.to_limbs());
        });
    }

    fn generate_challenge_field_element(&mut self) -> ChallengeF {
        self.fs_sponge.squeeze()
    }

    fn generate_challenge_u8_slice(&mut self, n_bytes: usize) -> Vec<u8> {
        let mut bytes = vec![];
        let mut buf = vec![];
        while bytes.len() < n_bytes {
            let digest = self.fs_sponge.squeeze();
            digest.serialize_into(&mut buf).unwrap();
            bytes.extend_from_slice(&buf);
        }
        bytes.resize(n_bytes, 0);
        bytes
    }

    fn finalize_and_get_proof(&self) -> Proof {
        self.proof.clone()
    }

    fn hash_and_return_state(&mut self) -> Vec<u8> {
        let state = self.fs_sponge.squeeze_state();
        let mut state_buffer = vec![];
        state.serialize_into(&mut state_buffer).unwrap();
        state_buffer
    }

    fn set_state(&mut self, state: &[u8]) {
        let new_state = State::deserialize_from(state).unwrap();
        self.fs_sponge.set_state(new_state)
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
