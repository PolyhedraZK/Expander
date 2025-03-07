use std::{fmt::Debug, io::Read, marker::PhantomData};

use arith::{ExtensionField, Field, FieldSerde};
use field_hashers::FiatShamirFieldHasher;

use crate::{fiat_shamir_hash::FiatShamirBytesHash, Proof};

// When appending the initial commitment, we hash the commitment bytes
// for sufficient number of times, so that the FS hash has a sufficient circuit depth

#[cfg(not(feature = "recursion"))]
const PCS_DIGEST_LOOP: usize = 1000;

pub trait Transcript<F: ExtensionField>: Clone + Debug {
    /// Create a new transcript.
    fn new() -> Self;

    /// Append a polynomial commitment to the transcript
    /// called by the prover
    fn append_commitment(&mut self, commitment_bytes: &[u8]);

    /// Append a polynomial commitment to the transcript
    /// check that the pcs digest in the proof is correct
    /// called by the verifier
    fn append_commitment_and_check_digest<R: Read>(
        &mut self,
        commitment_bytes: &[u8],
        proof_reader: &mut R,
    ) -> bool;

    /// Append a field element to the transcript.
    fn append_field_element(&mut self, f: &F);

    /// Append a slice of bytes
    fn append_u8_slice(&mut self, buffer: &[u8]);

    /// Generate a circuit field element.
    fn generate_circuit_field_element(&mut self) -> F::BaseField;

    /// Generate a slice of random circuit fields.
    fn generate_circuit_field_elements(&mut self, num_elems: usize) -> Vec<F::BaseField> {
        let mut circuit_fs = Vec::with_capacity(num_elems);
        (0..num_elems).for_each(|_| circuit_fs.push(self.generate_circuit_field_element()));
        circuit_fs
    }

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

impl<F: ExtensionField, H: FiatShamirBytesHash> Transcript<F> for BytesHashTranscript<F, H> {
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

    #[inline]
    fn append_commitment(&mut self, commitment_bytes: &[u8]) {
        self.append_u8_slice(commitment_bytes);

        #[cfg(not(feature = "recursion"))]
        {
            // When appending the initial commitment, we hash the commitment bytes
            // for sufficient number of times, so that the FS hash has a sufficient circuit depth
            let mut digest = [0u8; 32];
            H::hash(&mut digest, commitment_bytes);
            for _ in 0..PCS_DIGEST_LOOP {
                H::hash_inplace(&mut digest);
            }
            self.append_u8_slice(&digest);
        }
    }

    #[inline]
    /// check that the pcs digest in the proof is correct
    fn append_commitment_and_check_digest<R: Read>(
        &mut self,
        commitment_bytes: &[u8],
        _proof_reader: &mut R,
    ) -> bool {
        self.append_u8_slice(commitment_bytes);

        #[cfg(not(feature = "recursion"))]
        {
            // When appending the initial commitment, we hash the commitment bytes
            // for sufficient number of times, so that the FS hash has a sufficient circuit depth
            let mut digest = [0u8; 32];
            H::hash(&mut digest, commitment_bytes);
            for _ in 0..PCS_DIGEST_LOOP {
                H::hash_inplace(&mut digest);
            }
            self.append_u8_slice(&digest);

            // check that digest matches the proof
            let mut pcs_digest = [0u8; 32];
            _proof_reader.read_exact(&mut pcs_digest).unwrap();

            digest == pcs_digest
        }

        #[cfg(feature = "recursion")]
        true
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

    /// Generate a random circuit field.
    fn generate_circuit_field_element(&mut self) -> <F as ExtensionField>::BaseField {
        // NOTE(HS) fast path for BN254 sampling - notice that the deserialize from for BN254
        // does not self correct the sampling bytes, we instead use challenge field sampling
        // and cast it back to base field for BN254 case - maybe traverse back and unify the impls.
        if F::DEGREE == 1 {
            let challenge_fs = self.generate_challenge_field_element();
            return challenge_fs.to_limbs()[0];
        }

        let bytes_sampled = self.generate_challenge_u8_slice(32);
        let mut buffer = [0u8; 32];
        buffer[..32].copy_from_slice(&bytes_sampled);

        <F as ExtensionField>::BaseField::from_uniform_bytes(&buffer)
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

        ret.truncate(n_bytes);
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
            let hash_inputs = {
                let mut res = self.digest.clone();
                res.extend_from_slice(&self.proof.bytes[self.hash_start_index..hash_end_index]);
                res
            };

            H::hash(&mut self.digest, &hash_inputs);
            self.hash_start_index = hash_end_index;
        } else {
            H::hash_inplace(&mut self.digest);
        }
    }
}

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
            self.append_u8_slice(&digest_bytes);
        }
    }

    #[inline]
    fn append_commitment_and_check_digest<R: Read>(
        &mut self,
        commitment_bytes: &[u8],
        _proof_reader: &mut R,
    ) -> bool {
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
            self.append_u8_slice(&digest_bytes);

            // check that digest matches the proof
            let challenge_from_proof =
                Vec::<ChallengeF::BaseField>::deserialize_from(_proof_reader).unwrap();

            challenge_from_proof == challenge
        }

        #[cfg(feature = "recursion")]
        true
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
