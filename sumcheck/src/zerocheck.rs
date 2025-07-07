//! credit: https://github.com/EspressoSystems/hyperplonk/blob/main/subroutines/src/poly_iop/zero_check/mod.rs

use arith::Field;
use gkr_engine::Transcript;
use polynomials::{EqPolynomial, SumOfProductsPoly};

use crate::{IOPProof, SumCheck};

/// A zero check IOP subclaim for `f(x)` consists of the following:
///   - the initial challenge vector r which is used to build eq(x, r) in SumCheck
///   - the random vector `v` to be evaluated
///   - the claimed evaluation of `f(v)`
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ZeroCheckSubClaim<F: Field> {
    // the evaluation point
    pub point: Vec<F>,
    /// the expected evaluation
    pub expected_evaluation: F,
    // the initial challenge r which is used to build eq(x, r)
    pub init_challenge: Vec<F>,
}

pub struct ZeroCheck<F: Field> {
    phantom: std::marker::PhantomData<F>,
}

impl<F: Field> ZeroCheck<F> {
    pub fn prove(
        poly_list: &SumOfProductsPoly<F>,
        transcript: &mut impl Transcript,
    ) -> IOPProof<F> {
        let r = transcript.generate_field_elements::<F>(poly_list.num_vars());
        let zero_poly = poly_list.build_f_hat(&r);
        SumCheck::prove(&zero_poly, transcript)
    }

    pub fn verify(
        proof: &IOPProof<F>,
        transcript: &mut impl Transcript,
    ) -> (bool, ZeroCheckSubClaim<F>) {
        // check that the sum is zero
        if proof.proofs[0].evaluations[0] + proof.proofs[0].evaluations[1] != F::zero() {
            println!("ZeroCheck failed: sum is not zero: {:?}", proof.proofs[0].evaluations[0] + proof.proofs[0].evaluations[1]);
            return (
                false,
                ZeroCheckSubClaim {
                    point: vec![],
                    expected_evaluation: F::zero(),
                    init_challenge: vec![],
                },
            );
        }

        let num_vars = proof.point.len();
        let r = transcript.generate_field_elements::<F>(num_vars);
        let (verified, sumcheck_subclaim) =
            SumCheck::verify(F::zero(), proof, num_vars, transcript);

        if !verified {
            return (
                false,
                ZeroCheckSubClaim {
                    point: vec![],
                    expected_evaluation: F::zero(),
                    init_challenge: r,
                },
            );
        }

        // expected_eval = sumcheck.expect_eval/eq(v, r)
        // where v = sum_check_sub_claim.point
        let eq_x_r_eval = EqPolynomial::eq_vec(&sumcheck_subclaim.point, &r);
        let expected_evaluation =
            sumcheck_subclaim.expected_evaluation * eq_x_r_eval.inv().unwrap();

        (
            true,
            ZeroCheckSubClaim {
                point: sumcheck_subclaim.point,
                expected_evaluation,
                init_challenge: r,
            },
        )
    }
}
