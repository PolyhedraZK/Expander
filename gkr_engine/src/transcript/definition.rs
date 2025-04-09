use std::{fmt::Debug, io::Read, str::FromStr};

use arith::ExtensionField;

use crate::ExpErrors;

use super::Proof;

/// A trait for transcript generation over the challenge field
/// The associated field is the challenge field, i.e., M31Ext3
/// The challenge field is not SIMD enabled
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
        (0..num_elems)
            .map(|_| self.generate_circuit_field_element())
            .collect()
    }

    /// Generate a challenge.
    fn generate_challenge_field_element(&mut self) -> F;

    /// Generate a slice of random bytes of some fixed size
    /// Use this function when you need some randomness other than the native field
    fn generate_challenge_u8_slice(&mut self, n_bytes: usize) -> Vec<u8>;

    /// Generate a list of positions that we want to open the polynomial at.
    #[inline]
    fn generate_challenge_index_vector(&mut self, num_queries: usize) -> Vec<usize> {
        (0..num_queries)
            .map(|_| usize::from_le_bytes(self.generate_challenge_u8_slice(8).try_into().unwrap()))
            .collect()
    }

    /// Generate a challenge vector.
    #[inline]
    fn generate_challenge_field_elements(&mut self, n: usize) -> Vec<F> {
        (0..n)
            .map(|_| self.generate_challenge_field_element())
            .collect()
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

#[derive(Debug, Clone, PartialEq, Default)]
pub enum FiatShamirHashType {
    #[default]
    SHA256,
    Keccak256,
    Poseidon,
    Animoe,
    MIMC5, // Note: use MIMC5 for bn254 ONLY
}

impl FromStr for FiatShamirHashType {
    type Err = ExpErrors;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SHA256" => Ok(FiatShamirHashType::SHA256),
            "Keccak256" => Ok(FiatShamirHashType::Keccak256),
            "Poseidon" => Ok(FiatShamirHashType::Poseidon),
            "Animoe" => Ok(FiatShamirHashType::Animoe),
            "MIMC5" => Ok(FiatShamirHashType::MIMC5),
            _ => Err(ExpErrors::FiatShamirHashTypeError(s.to_string())),
        }
    }
}
