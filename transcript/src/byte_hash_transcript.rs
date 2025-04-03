use std::{fmt::Debug, io::Read, marker::PhantomData};

use arith::{ExtensionField, Field};
use gkr_engine::{Proof, Transcript};
use gkr_hashers::FiatShamirBytesHash;

// When appending the initial commitment, we hash the commitment bytes
// for sufficient number of times, so that the FS hash has a sufficient circuit depth

#[cfg(not(feature = "recursion"))]
const PCS_DIGEST_LOOP: usize = 1000;

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
