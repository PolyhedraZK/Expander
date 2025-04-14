use std::{fmt::Debug, io::Read};

use arith::Field;
use gkr_engine::{Proof, Transcript};
use gkr_hashers::FiatShamirHasher;

// When appending the initial commitment, we hash the commitment bytes
// for sufficient number of times, so that the FS hash has a sufficient circuit depth

#[cfg(not(feature = "recursion"))]
const PCS_DIGEST_LOOP: usize = 1000;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct FieldHashTranscript<H>
where
    H: FiatShamirHasher,
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
    H: FiatShamirHasher,
{
    #[inline(always)]
    fn new() -> Self {
        Self {
            hasher: H::new(),
            digest: vec![0u8; H::DIGEST_SIZE],
            digest_start: H::DIGEST_SIZE,
            proof: Proof::default(),
            hash_start_index: 0,
            proof_locked: false,
            proof_locked_at: 0,
        }
    }

    #[inline]
    fn init_commitment<F: Field>(&mut self, _commitment_bytes: &[u8]) -> Vec<u8>{
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
        self.proof.bytes.extend_from_slice(buffer);
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