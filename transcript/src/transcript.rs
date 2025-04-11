use std::{fmt::Debug, io::Read, marker::PhantomData};

use arith::{ExtensionField, Field};
use crate::FiatShamirHash;
use serdes::ExpSerde;

use crate::Proof;

// When appending the initial commitment, we hash the commitment bytes
// for sufficient number of times, so that the FS hash has a sufficient circuit depth

#[cfg(not(feature = "recursion"))]
const PCS_DIGEST_LOOP: usize = 1000;

pub trait Transcript: Clone + Debug {
    /// Create a new transcript.
    fn new() -> Self;

    fn init_commitment<F: Field>(&mut self, commitment_bytes: &[u8]) -> Vec<u8>;

    /// Append a polynomial commitment to the transcript
    /// called by the prover
    fn append_commitment<F: Field>(&mut self, commitment_bytes: &[u8]);

    /// Append a polynomial commitment to the transcript
    /// check that the pcs digest in the proof is correct
    /// called by the verifier
    fn append_commitment_and_check_digest<F: Field, R: Read>(
        &mut self,
        commitment_bytes: &[u8],
        proof_reader: &mut R,
    ) -> bool;

    /// Append a field element to the transcript.
    #[inline]
    fn append_field_element<F: Field>(&mut self, f: &F) {
        let mut buf = vec![];
        f.serialize_into(&mut buf).unwrap();
        self.append_u8_slice(&buf);
    }

    /// Append a slice of bytes
    fn append_u8_slice(&mut self, buffer: &[u8]);
    /*
        self.proof.bytes.extend_from_slice(buffer);
    */

    /// Generate a slice of random bytes.
    fn generate_u8_slice(&mut self, n_bytes: usize) -> Vec<u8>;

    fn generate_usize_vector(&mut self, n: usize) -> Vec<usize> {
        let mut res: Vec<usize> = vec![0; n];
        let mut buf = [0u8; 8];
        for e in res.iter_mut() {
            buf.copy_from_slice(&self.generate_u8_slice(8));
            *e = usize::from_le_bytes(buf);
        }
        res
    }

    /// Generate a field element.
    #[inline(always)]
    fn generate_field_element<F: Field>(&mut self) -> F {
        F::from_uniform_bytes(&self.generate_u8_slice(F::SIZE))
    }

    /// Generate a field element vector.
    #[inline(always)]
    fn generate_field_elements<F: Field>(&mut self, n: usize) -> Vec<F> {
        let mut res = vec![F::ZERO; n];
        for e in res.iter_mut() {
            *e = self.generate_field_element();
        }
        res
    }

    /// Produce the proof
    /// It is not recommended to append/challenge after calling this function
    fn finalize_and_get_proof(&mut self) -> Proof;

    /// Return current state of the transcript
    /// Note: this may incur an additional hash to shrink the state
    fn hash_and_return_state(&mut self) -> Vec<u8>;
    /*
        self.refresh_digest();
        self.digets.clone();
    */

    /// Set the state
    /// Note: Any unhashed data will be discarded
    fn set_state(&mut self, state: &[u8]);
    /*
        self.hash_start_index = self.proof.bytes.len(); // discard unhashed data
        assert!(state.len() == H::DIGEST_SIZE);
        self.digest = state.to_vec();
    */

    /// lock proof, no changes will be made to the proof until unlock is called
    fn lock_proof(&mut self);
    /*
        assert!(!self.proof_locked);
        self.proof_locked = true;
        self.proof_locked_at = self.proof.bytes.len();
    */

    /// unlock proof
    fn unlock_proof(&mut self);

    fn refresh_digest(&mut self);

    fn get_digest_start(&mut self) -> usize;
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct BytesHashTranscript<H: FiatShamirHash> {
    hasher: H,

    /// The digest bytes.
    pub digest: Vec<u8>,
    digest_start: usize,

    /// The proof bytes.
    proof: Proof,

    /// The pointer to the proof bytes indicating where the hash starts.
    hash_start_index: usize,

    /// locking point
    proof_locked: bool,
    proof_locked_at: usize,
}

impl<H: FiatShamirHash> Transcript for BytesHashTranscript<H> {
    fn new() -> Self {
        Self {
            hasher: H::new(),
            digest: vec![0u8; H::DIGEST_SIZE],
            digest_start: 0,
            proof: Proof::default(),
            hash_start_index: 0,
            proof_locked: false,
            proof_locked_at: 0,
        }
    }

    #[inline(always)]
    fn init_commitment<F: Field>(&mut self, commitment_bytes: &[u8]) -> Vec<u8> {
        let mut digest = vec![0u8; H::DIGEST_SIZE];
        self.hasher.hash(&mut digest, commitment_bytes);
        for _ in 0..PCS_DIGEST_LOOP {
            self.hasher.hash_inplace(&mut digest);
        }
        digest
    }

    #[inline]
    fn append_commitment<F: Field>(&mut self, commitment_bytes: &[u8]) {
        self.append_u8_slice(commitment_bytes);

        #[cfg(not(feature = "recursion"))]
        {
            // When appending the initial commitment, we hash the commitment bytes
            // for sufficient number of times, so that the FS hash has a sufficient circuit depth
            let digest = self.init_commitment::<F>(commitment_bytes);
            self.append_u8_slice(&digest);
        }
    }

    #[inline]
    /// check that the pcs digest in the proof is correct
    fn append_commitment_and_check_digest<F: Field, R: Read>(
        &mut self,
        commitment_bytes: &[u8],
        _proof_reader: &mut R,
    ) -> bool {
        self.append_u8_slice(commitment_bytes);

        #[cfg(not(feature = "recursion"))]
        {
            // When appending the initial commitment, we hash the commitment bytes
            // for sufficient number of times, so that the FS hash has a sufficient circuit depth
            let digest = self.init_commitment::<F>(commitment_bytes);
            self.append_u8_slice(&digest);

            // check that digest matches the proof
            let mut pcs_digest = [0u8; 32];
            _proof_reader.read_exact(&mut pcs_digest).unwrap();

            digest == pcs_digest
        }

        #[cfg(feature = "recursion")]
        true
    }

    /// Append a byte slice to the transcript.
    #[inline(always)]
    fn append_u8_slice(&mut self, buffer: &[u8]) {
        self.proof.bytes.extend_from_slice(buffer);
    }

    #[inline]
    fn generate_u8_slice(&mut self, n_bytes: usize) -> Vec<u8> {
        let mut ret = vec![0u8; n_bytes];
        let mut cur_n_bytes = 0usize;

        while cur_n_bytes < n_bytes {
            let digest_start = self.get_digest_start();
            let digest_len = H::DIGEST_SIZE.min(n_bytes - cur_n_bytes);
            ret[cur_n_bytes..cur_n_bytes + digest_len].copy_from_slice(&self.digest[digest_start..digest_start + digest_len]);
            cur_n_bytes += digest_len;
            self.digest_start += digest_len;
        }

        ret
    }

    #[inline(always)]
    fn finalize_and_get_proof(&mut self) -> Proof {
        if self.proof_locked {
            self.unlock_proof();
        }
        self.proof.clone()
    }

    #[inline(always)]
    fn hash_and_return_state(&mut self) -> Vec<u8> {
        self.refresh_digest();
        self.digest.clone()
    }

    #[inline(always)]
    fn set_state(&mut self, state: &[u8]) {
        self.hash_start_index = self.proof.bytes.len(); // discard unhashed data
        assert!(state.len() == H::DIGEST_SIZE);
        self.digest = state.to_vec();
    }

    #[inline(always)]
    fn lock_proof(&mut self) {
        assert!(!self.proof_locked);
        self.proof_locked = true;
        self.proof_locked_at = self.proof.bytes.len();
    }

    #[inline(always)]
    fn unlock_proof(&mut self) {
        assert!(self.proof_locked);
        self.proof_locked = false;
        if self.hash_start_index < self.proof.bytes.len() {
            self.refresh_digest();
        }
        self.proof.bytes.resize(self.proof_locked_at, 0);
        self.hash_start_index = self.proof.bytes.len();
    }

    #[inline]
    fn refresh_digest(&mut self) {
        let hash_end_index = self.proof.bytes.len();
        if hash_end_index > self.hash_start_index {
            let hash_inputs = {
                let mut res = self.digest.clone();
                res.extend_from_slice(&self.proof.bytes[self.hash_start_index..hash_end_index]);
                res
            };

            self.hasher.hash(&mut self.digest, &hash_inputs);
            self.hash_start_index = hash_end_index;
        } else {
            self.hasher.hash_inplace(&mut self.digest);
        }
        self.digest_start = 0;
    }

    fn get_digest_start(&mut self) -> usize {
        if self.digest_start >= H::DIGEST_SIZE {
            self.refresh_digest();
        }
        self.digest_start
    }
    
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct FieldHashTranscript<H>
where
    H: FiatShamirHash,
{
    /// Internal hasher, it's a little costly to create a new hasher
    pub hasher: H,

    pub digest: Vec<u8>,
    digest_start: usize,

    /// The proof bytes.
    proof: Proof,

    /// The pointer to the proof bytes indicating where the hash starts.
    hash_start_index: usize,

    /// locking point
    proof_locked: bool,
    proof_locked_at: usize,
}

impl<H> Transcript for FieldHashTranscript<H>
where
    H: FiatShamirHash,
{
    #[inline(always)]
    fn new() -> Self {
        Self {
            hasher: H::new(),
            digest: vec![0u8; H::DIGEST_SIZE],
            digest_start: 0,
            proof: Proof::default(),
            hash_start_index: 0,
            proof_locked: false,
            proof_locked_at: 0,
        }
    }

    #[inline]
    fn init_commitment<F: Field>(&mut self, commitment_bytes: &[u8]) -> Vec<u8>{
        let challenge = self.generate_u8_slice(F::SIZE);
        let mut digest = vec![0u8; H::DIGEST_SIZE];
        self.hasher.hash(&mut digest, &challenge);
        for _ in 1..PCS_DIGEST_LOOP {
            self.hasher.hash_inplace(&mut digest);
        }
        digest
    }

    #[inline]
    fn append_commitment<F: Field>(&mut self, commitment_bytes: &[u8]) {
        self.append_u8_slice(commitment_bytes);

        #[cfg(not(feature = "recursion"))]
        {
            // When appending the initial commitment, we hash the commitment bytes
            // for sufficient number of times, so that the FS hash has a sufficient circuit depth
            let digest = self.init_commitment::<F>(commitment_bytes);

            self.append_u8_slice(&digest);
        }
    }

    #[inline]
    fn append_commitment_and_check_digest<F: Field, R: Read>(
        &mut self,
        commitment_bytes: &[u8],
        _proof_reader: &mut R,
    ) -> bool {
        self.append_u8_slice(commitment_bytes);

        #[cfg(not(feature = "recursion"))]
        {
            // When appending the initial commitment, we hash the commitment bytes
            // for sufficient number of times, so that the FS hash has a sufficient circuit depth
            let digest = self.init_commitment::<F>(commitment_bytes);

            self.append_u8_slice(&digest);

            // check that digest matches the proof
            let mut challenge_from_proof = vec![0u8; H::DIGEST_SIZE];
            _proof_reader.read_exact(&mut challenge_from_proof).unwrap();

            challenge_from_proof == digest
        }

        #[cfg(feature = "recursion")]
        true
    }

    fn append_u8_slice(&mut self, buffer: &[u8]) {
        if !self.proof_locked {
            self.proof.bytes.extend_from_slice(buffer);
        }
        self.digest.extend_from_slice(buffer);
    }

    fn generate_u8_slice(&mut self, n_bytes: usize) -> Vec<u8> {
        let mut ret = vec![0u8; n_bytes];
        let mut cur_n_bytes = 0usize;

        while cur_n_bytes < n_bytes {
            let digest_start = self.get_digest_start();
            let digest_len = H::DIGEST_SIZE.min(n_bytes - cur_n_bytes);
            ret[cur_n_bytes..cur_n_bytes + digest_len].copy_from_slice(&self.digest[digest_start..digest_start + digest_len]);
            cur_n_bytes += digest_len;
            self.digest_start += digest_len;
        }

        ret
    }

    #[inline]
    fn finalize_and_get_proof(&mut self) -> Proof {
        if self.proof_locked {
            self.unlock_proof();
        }
        self.proof.clone()
    }

    // NOTE(HS) the hash_and_return_state and set_state are always called together
    // mainly to sync up the MPI transcripts to a same transaction hash state.
    fn hash_and_return_state(&mut self) -> Vec<u8> {
        self.refresh_digest();
        self.digest.clone()
    }

    fn set_state(&mut self, state: &[u8]) {
        self.hash_start_index = self.proof.bytes.len(); // discard unhashed data
        assert!(state.len() == H::DIGEST_SIZE);
        self.digest = state.to_vec();
        // NOTE(HS) since we broadcasted, then all the hash state base field elems
        // are being observed, then all of them are consumed
    }

    #[inline(always)]
    fn lock_proof(&mut self) {
        assert!(!self.proof_locked);
        self.proof_locked = true;
    }

    #[inline(always)]
    fn unlock_proof(&mut self) {
        assert!(self.proof_locked);
        self.proof_locked = false;
        if self.hash_start_index < self.proof.bytes.len() {
            self.refresh_digest();
        }
        self.proof.bytes.resize(self.proof_locked_at, 0);
        self.hash_start_index = self.proof.bytes.len();
    }

    #[inline]
    fn refresh_digest(&mut self) {
        let hash_end_index = self.proof.bytes.len();
        if hash_end_index > self.hash_start_index {
            let hash_inputs = {
                let mut res = self.digest.clone();
                res.extend_from_slice(&self.proof.bytes[self.hash_start_index..hash_end_index]);
                res
            };

            self.hasher.hash(&mut self.digest, &hash_inputs);
            self.hash_start_index = hash_end_index;
        } else {
            self.hasher.hash_inplace(&mut self.digest);
        }
        self.digest_start = 0;
    }

    fn get_digest_start(&mut self) -> usize {
        if self.digest_start >= H::DIGEST_SIZE {
            self.refresh_digest();
        }
        self.digest_start
    }
}
