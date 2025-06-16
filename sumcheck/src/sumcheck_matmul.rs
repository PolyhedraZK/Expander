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
        let (sum_poly, c) = input.form_sumcheck_polynomial(transcript);
        let proof = SumCheck::prove(&sum_poly, transcript);

        let eval_c = c.eval_reverse_order(&proof.point);
        let eval_poly = sum_poly.evaluate(&proof.point);

        println!("c eval: {eval_c:?}");
        println!("poly eval: {eval_poly:?}");

        proof
    }

    #[inline]
    pub fn verify(
        claimed_sum: F,
        proof: &IOPProof<F>,
        num_vars: usize,
        transcript: &mut impl Transcript,
    ) -> SumCheckSubClaim<F> {
        let r = transcript.generate_field_element::<F>();
        println!("Verifying sumcheck with r = {r:?}");
        SumCheck::verify(claimed_sum, proof, num_vars, transcript)
    }
}
