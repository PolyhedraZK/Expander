use arith::Field;
use gkr_engine::Transcript;
use polynomials::MultiLinearPoly;

use crate::{IOPProof, SumCheck, SumCheckSubClaim, SumOfProductsPoly};

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
        let sum_poly = input.form_sumcheck_polynomial(transcript);
        SumCheck::prove(&sum_poly, transcript)
    }

    #[inline]
    pub fn verify(
        claimed_sum: F,
        proof: &IOPProof<F>,
        num_vars: usize,
        transcript: &mut impl Transcript,
    ) -> SumCheckSubClaim<F> {
        SumCheck::verify(claimed_sum, proof, num_vars, transcript)
    }
}
