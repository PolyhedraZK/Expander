use super::*;

use arith::Fr;
use ark_std::test_rng;
use gkr_hashers::Keccak256hasher;
use polynomials::{MultiLinearPoly, ProductOfMLEs};
use transcript::BytesHashTranscript;

#[test]
fn test_sumcheck_subroutine() {
    let mut rng = test_rng();

    for num_vars in 1..10 {
        let size = 1 << num_vars;

        for num_poly in 1..10 {
            for degree in 2..=3 {
                let monomials = (0..num_poly)
                    .map(|_| {
                        let polynomials = (0..degree)
                            .map(|_| {
                                // Generate random coefficients for f and g
                                // We can use Fr::rand to generate random elements in the field
                                let coeffs = (1..=size)
                                    .map(|_| Fr::random_unsafe(&mut rng))
                                    // .map(|i| Fr::from(i))
                                    .collect::<Vec<_>>();
                                MultiLinearPoly::<Fr>::new(coeffs)
                            })
                            .collect::<Vec<_>>();
                        ProductOfMLEs { polynomials }
                    })
                    .collect::<Vec<_>>();

                let mle_list = SumOfProductsPoly { monomials };

                let asserted_sum = mle_list.sum();

                let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();

                let mut prover = IOPProverState::prover_init(&mle_list);
                let mut verifier = IOPVerifierState::verifier_init(prover.mle_list.num_vars());

                let mut challenge = None;

                for _ in 0..prover.mle_list.num_vars() {
                    let prover_msg =
                        IOPProverState::prove_round_and_update_state(&mut prover, &challenge);

                    challenge = Some(IOPVerifierState::verify_round_and_update_state(
                        &mut verifier,
                        &prover_msg,
                        &mut transcript,
                    ));
                }

                let (verified, subclaim) =
                    IOPVerifierState::check_and_generate_subclaim(&verifier, &asserted_sum);
                assert!(verified, "sumcheck verification failed");
                let evals = mle_list.evaluate(&subclaim.point);
                assert!(evals == subclaim.expected_evaluation, "wrong subclaim");
            }
        }
    }
}

#[test]
fn test_sumcheck_e2e() {
    let mut rng = test_rng();

    for num_vars in 1..10 {
        let size = 1 << num_vars;

        for num_poly in 1..10 {
            for degree in 2..=3 {
                let monomials = (0..num_poly)
                    .map(|_| {
                        let polynomials = (0..degree)
                            .map(|_| {
                                // Generate random coefficients for f and g
                                // We can use Fr::rand to generate random elements in the field
                                let coeffs = (1..=size)
                                    .map(|_| Fr::random_unsafe(&mut rng))
                                    // .map(|i| Fr::from(i))
                                    .collect::<Vec<_>>();
                                MultiLinearPoly::<Fr>::new(coeffs)
                            })
                            .collect::<Vec<_>>();
                        ProductOfMLEs { polynomials }
                    })
                    .collect::<Vec<_>>();

                let mle_list = SumOfProductsPoly { monomials };

                let asserted_sum = mle_list.sum();

                // prover
                let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
                let proof = SumCheck::<Fr>::prove(&mle_list, &mut transcript);

                // verifier
                let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
                let (verified, subclaim) =
                    SumCheck::<Fr>::verify(asserted_sum, &proof, num_vars, &mut transcript);
                assert!(verified, "sumcheck verification failed");
                let evals = mle_list.evaluate(&subclaim.point);
                assert!(evals == subclaim.expected_evaluation, "wrong subclaim");
            }
        }
    }
}
