use std::{fmt::Debug, io::Read};

use arith::ExtensionField;

pub trait Transcript<F: ExtensionField>: Clone + Debug {
    /// The proof type
    type Proof: Clone + Debug;

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
    fn finalize_and_get_proof(&self) -> Self::Proof;

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
