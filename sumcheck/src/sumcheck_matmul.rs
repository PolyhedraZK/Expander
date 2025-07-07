use arith::Field;
use gkr_engine::Transcript;

use crate::{IOPProof, SumCheck, ZeroCheck, ZeroCheckSubClaim};

mod matrix;
pub use matrix::*;

mod witnesses;
pub use witnesses::*;

#[cfg(test)]
mod tests;

pub struct SumCheckMatMul<F: Field> {
    phantom: std::marker::PhantomData<F>,
}

impl<F: Field> SumCheckMatMul<F> {
    /// Extract sum from the proof
    pub fn extract_sum(proof: &IOPProof<F>) -> F {
        SumCheck::extract_sum(proof)
    }

    #[inline]
    pub fn prove(input: &MatMulWitnesses<F>, transcript: &mut impl Transcript) -> IOPProof<F> {
        let (sum_poly, c) = input.form_zerocheck_polynomial(transcript);
        ZeroCheck::prove(&sum_poly, transcript)
    }

    #[inline]
    pub fn verify(
        proof: &IOPProof<F>,
        transcript: &mut impl Transcript,
    ) -> (bool, ZeroCheckSubClaim<F>) {
        // todo: check r is correct
        let _r = transcript.generate_field_element::<F>();
        ZeroCheck::verify(proof, transcript)
    }
}
