use super::*;

use arith::Fr;
use ark_std::test_rng;
use gkr_hashers::Keccak256hasher;
use polynomials::MultilinearExtension;
use transcript::BytesHashTranscript;

#[test]
fn test_sumcheck_subroutine() {
    for num_vars in 1..10 {
        let size = 1 << num_vars;

        for num_poly in 1..10 {
            let mle_list = (0..num_poly)
                .map(|j| {
                    let coeffs = (1..=size)
                        .map(|i| Fr::from(j * 10 + i as u32))
                        .collect::<Vec<_>>();
                    MultiLinearPoly::<Fr>::new(coeffs)
                })
                .collect::<Vec<_>>();

            let asserted_sum = mle_list
                .iter()
                .map(|mle| mle.coeffs.iter().sum::<Fr>())
                .sum::<Fr>();

            let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();

            let mut prover = IOPProverState::prover_init(&mle_list);
            let mut verifier = IOPVerifierState::verifier_init(prover.mle_list[0].num_vars());

            let mut challenge = None;

            for _ in 0..prover.mle_list[0].num_vars() {
                let prover_msg =
                    IOPProverState::prove_round_and_update_state(&mut prover, &challenge);

                challenge = Some(IOPVerifierState::verify_round_and_update_state(
                    &mut verifier,
                    &prover_msg,
                    &mut transcript,
                ));
            }

            let subclaim = IOPVerifierState::check_and_generate_subclaim(&verifier, &asserted_sum);

            let evals = mle_list
                .iter()
                .map(|mle| mle.eval_reverse_order(&subclaim.point))
                .sum::<Fr>();

            assert!(
                evals == subclaim.expected_evaluation,
                "wrong subclaim: {:?} != {:?}",
                evals,
                subclaim.expected_evaluation
            );
        }
    }
}

#[test]
fn test_sumcheck_e2e() {
    let mut rng = test_rng();

    for num_vars in 1..10 {
        let size = 1 << num_vars;

        for num_poly in 1..10 {
            let mle_list = (0..num_poly)
                .map(|_| {
                    let coeffs = (1..=size)
                        .map(|_| Fr::random_unsafe(&mut rng))
                        .collect::<Vec<_>>();
                    MultiLinearPoly::<Fr>::new(coeffs)
                })
                .collect::<Vec<_>>();

            let asserted_sum = mle_list
                .iter()
                .map(|mle| mle.coeffs.iter().sum::<Fr>())
                .sum::<Fr>();

            // prover
            let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
            let proof = SumCheck::<Fr>::prove(&mle_list, &mut transcript);

            // verifier
            let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
            let subclaim = SumCheck::<Fr>::verify(asserted_sum, &proof, num_vars, &mut transcript);

            let evals = mle_list
                .iter()
                .map(|mle| mle.eval_reverse_order(&subclaim.point))
                .sum::<Fr>();

            assert!(evals == subclaim.expected_evaluation, "wrong subclaim");
        }
    }
}
