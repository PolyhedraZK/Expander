use std::fmt::Debug;

use arith::{ExtensionField, Field};
use gkr_engine::{Proof, Transcript};
use gkr_hashers::FiatShamirFieldHasher;
use serdes::ExpSerde;

// When appending the initial commitment, we hash the commitment bytes
// for sufficient number of times, so that the FS hash has a sufficient circuit depth

#[cfg(not(feature = "recursion"))]
const PCS_DIGEST_LOOP: usize = 1000;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct FieldHashTranscript<ChallengeF, H>
where
    ChallengeF: ExtensionField,
    H: FiatShamirFieldHasher<ChallengeF::BaseField>,
{
    /// Internal hasher, it's a little costly to create a new hasher
    pub hasher: H,

    /// The hash state, a vec of base field elems up to H::STATE_CAPACITY.
    pub hash_state: Vec<ChallengeF::BaseField>,

    /// The index pointing into hash state, for the next unconsumed base field hash elem.
    pub next_unconsumed: usize,

    /// The proof bytes
    pub proof: Proof,

    /// The data to be hashed
    pub data_pool: Vec<ChallengeF::BaseField>,

    /// Proof locked or not
    pub proof_locked: bool,
}

impl<ChallengeF, H> Transcript<ChallengeF> for FieldHashTranscript<ChallengeF, H>
where
    ChallengeF: ExtensionField,
    H: FiatShamirFieldHasher<ChallengeF::BaseField>,
{
    #[inline(always)]
    fn new() -> Self {
        Self {
            hasher: H::new(),
            ..Default::default()
        }
    }

    #[inline]
    fn append_commitment(&mut self, commitment_bytes: &[u8]) {
        self.append_u8_slice(commitment_bytes);

        #[cfg(not(feature = "recursion"))]
        {
            // When appending the initial commitment, we hash the commitment bytes
            // for sufficient number of times, so that the FS hash has a sufficient circuit depth
            let mut challenge = self.generate_challenge_field_element().to_limbs();
            let hasher = H::new();
            for _ in 0..PCS_DIGEST_LOOP {
                challenge = hasher.hash_to_state(&challenge);
            }

            let mut digest_bytes = vec![];
            challenge.serialize_into(&mut digest_bytes).unwrap();
            self.set_state(&digest_bytes);
        }
    }

    fn append_field_element(&mut self, f: &ChallengeF) {
        if !self.proof_locked {
            let mut buffer = vec![];
            f.serialize_into(&mut buffer).unwrap();
            self.proof.bytes.extend_from_slice(&buffer);
        }
        self.data_pool.extend_from_slice(&f.to_limbs());
    }

    fn append_u8_slice(&mut self, buffer: &[u8]) {
        if !self.proof_locked {
            self.proof.bytes.extend_from_slice(buffer);
        }
        let mut local_buffer = buffer.to_vec();
        local_buffer.resize(local_buffer.len().next_multiple_of(32), 0u8);
        local_buffer.chunks(32).for_each(|chunk_32| {
            let challenge_f = ChallengeF::from_uniform_bytes(chunk_32.try_into().unwrap());
            self.data_pool.extend_from_slice(&challenge_f.to_limbs());
        });
    }

    fn generate_circuit_field_element(&mut self) -> <ChallengeF as ExtensionField>::BaseField {
        if !self.data_pool.is_empty() {
            self.hash_state.extend_from_slice(&self.data_pool);
            self.hash_state = self.hasher.hash_to_state(&self.hash_state);
            self.data_pool.clear();
            self.next_unconsumed = 0;
        }

        if self.next_unconsumed < H::STATE_CAPACITY {
            let circuit_f = self.hash_state[self.next_unconsumed];
            self.next_unconsumed += 1;
            return circuit_f;
        }

        self.hash_state = self.hasher.hash_to_state(&self.hash_state);
        let circuit_f = self.hash_state[0];
        self.next_unconsumed = 1;

        circuit_f
    }

    fn generate_challenge_field_element(&mut self) -> ChallengeF {
        if !self.data_pool.is_empty() {
            self.hash_state.extend_from_slice(&self.data_pool);
            self.hash_state = self.hasher.hash_to_state(&self.hash_state);
            self.data_pool.clear();
            self.next_unconsumed = 0;
        }

        if ChallengeF::DEGREE + self.next_unconsumed <= H::STATE_CAPACITY {
            let challenge_f = ChallengeF::from_limbs(&self.hash_state[self.next_unconsumed..]);
            self.next_unconsumed += ChallengeF::DEGREE;
            return challenge_f;
        }

        let mut ext_limbs = Vec::new();
        if self.next_unconsumed < H::STATE_CAPACITY {
            ext_limbs.extend_from_slice(&self.hash_state[self.next_unconsumed..H::STATE_CAPACITY]);
        }
        let remaining_base_elems = ChallengeF::DEGREE - ext_limbs.len();

        self.hash_state = self.hasher.hash_to_state(&self.hash_state);
        ext_limbs.extend_from_slice(&self.hash_state[..remaining_base_elems]);
        self.next_unconsumed = remaining_base_elems;

        ChallengeF::from_limbs(&ext_limbs)
    }

    fn generate_challenge_u8_slice(&mut self, n_bytes: usize) -> Vec<u8> {
        let base_field_elems_num =
            (n_bytes + ChallengeF::BaseField::SIZE - 1) / ChallengeF::BaseField::SIZE;
        let elems = self.generate_challenge_field_elements(base_field_elems_num);

        let mut res = Vec::new();
        let mut buffer = Vec::new();

        elems.iter().for_each(|t| {
            t.serialize_into(&mut buffer).unwrap();
            res.extend_from_slice(&buffer);
        });
        res.truncate(n_bytes);

        res
    }

    fn finalize_and_get_proof(&self) -> Proof {
        self.proof.clone()
    }

    // NOTE(HS) the hash_and_return_state and set_state are always called together
    // mainly to sync up the MPI transcripts to a same transaction hash state.
    fn hash_and_return_state(&mut self) -> Vec<u8> {
        if !self.data_pool.is_empty() {
            self.hash_state = self.hasher.hash_to_state(&self.data_pool);
            self.data_pool.clear();
        } else {
            self.hash_state = self.hasher.hash_to_state(&self.hash_state);
        }

        let mut state = vec![];
        self.hash_state.serialize_into(&mut state).unwrap();
        state
    }

    fn set_state(&mut self, state: &[u8]) {
        assert!(self.data_pool.is_empty());
        // NOTE(HS) since we broadcasted, then all the hash state base field elems
        // are being observed, then all of them are consumed
        self.next_unconsumed = H::STATE_CAPACITY;
        self.hash_state = Vec::deserialize_from(state).unwrap();
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
