use std::sync::Arc;

use arith::{Field, Fr};
use ark_std::test_rng;
use gkr_engine::Transcript;
use gkr_hashers::Keccak256hasher;
use polynomials::{MultiLinearPoly, UnivariatePoly, VirtualPolynomial};
use transcript::BytesHashTranscript;

use crate::{interpolate_uni_poly, GenericSumcheck, ProverState, VerifierState};

#[test]
fn test_interpolation() {
    let mut prng = test_rng();

    // test a polynomial with 20 known points, i.e., with degree 19
    let poly = UnivariatePoly::<Fr>::random(20 - 1, &mut prng);
    let evals = (0u32..20)
        .map(|i| poly.evaluate(Fr::from(i)))
        .collect::<Vec<Fr>>();
    let query = Fr::random_unsafe(&mut prng);

    assert_eq!(poly.evaluate(query), interpolate_uni_poly(&evals, query));

    // test a polynomial with 33 known points, i.e., with degree 32
    let poly = UnivariatePoly::<Fr>::random(33 - 1, &mut prng);
    let evals = (0u32..33)
        .map(|i| poly.evaluate(Fr::from(i)))
        .collect::<Vec<Fr>>();
    let query = Fr::random_unsafe(&mut prng);

    assert_eq!(poly.evaluate(query), interpolate_uni_poly(&evals, query));

    // test a polynomial with 64 known points, i.e., with degree 63
    let poly = UnivariatePoly::<Fr>::random(64 - 1, &mut prng);
    let evals = (0u32..64)
        .map(|i| poly.evaluate(Fr::from(i)))
        .collect::<Vec<Fr>>();
    let query = Fr::random_unsafe(&mut prng);

    assert_eq!(poly.evaluate(query), interpolate_uni_poly(&evals, query));
}

fn test_sumcheck(nv: usize, num_multiplicands_range: (usize, usize), num_products: usize) {
    let mut rng = test_rng();
    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();

    let (poly, asserted_sum) =
        VirtualPolynomial::rand(nv, num_multiplicands_range, num_products, &mut rng);
    let proof = GenericSumcheck::<Fr>::prove(&poly, &mut transcript);
    let poly_info = poly.aux_info.clone();

    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let subclaim = GenericSumcheck::<Fr>::verify(asserted_sum, &proof, &poly_info, &mut transcript);
    assert!(
        poly.evaluate(&subclaim.point) == subclaim.expected_evaluation,
        "wrong subclaim"
    );
}

fn test_sumcheck_internal(nv: usize, num_multiplicands_range: (usize, usize), num_products: usize) {
    let mut rng = test_rng();
    let (poly, asserted_sum) =
        VirtualPolynomial::<Fr>::rand(nv, num_multiplicands_range, num_products, &mut rng);
    let poly_info = poly.aux_info.clone();
    let mut prover_state = ProverState::prover_init(&poly);
    let mut verifier_state = VerifierState::verifier_init(&poly_info);
    let mut challenge = None;
    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();

    for _ in 0..poly.aux_info.num_variables {
        let prover_message =
            ProverState::prove_round_and_update_state(&mut prover_state, &challenge);

        challenge = Some(VerifierState::verify_round_and_update_state(
            &mut verifier_state,
            &prover_message,
            &mut transcript,
        ));
    }
    let subclaim = VerifierState::check_and_generate_subclaim(&verifier_state, &asserted_sum);
    assert!(
        poly.evaluate(&subclaim.point) == subclaim.expected_evaluation,
        "wrong subclaim"
    );
}

#[test]
fn test_trivial_polynomial() {
    let nv = 1;
    let num_multiplicands_range = (4, 13);
    let num_products = 5;

    test_sumcheck(nv, num_multiplicands_range, num_products);
    test_sumcheck_internal(nv, num_multiplicands_range, num_products)
}
#[test]
fn test_normal_polynomial() {
    let nv = 3;
    let num_multiplicands_range = (1, 2);
    let num_products = 5;

    // test_sumcheck(nv, num_multiplicands_range, num_products);
    test_sumcheck_internal(nv, num_multiplicands_range, num_products)
}

#[test]
fn test_extract_sum() {
    let mut rng = test_rng();
    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let (poly, asserted_sum) = VirtualPolynomial::<Fr>::rand(8, (3, 4), 3, &mut rng);

    let proof = GenericSumcheck::<Fr>::prove(&poly, &mut transcript);
    assert_eq!(GenericSumcheck::<Fr>::extract_sum(&proof), asserted_sum);
}

// #[test]
// /// Test that the memory usage of shared-reference is linear to number of
// /// unique MLExtensions instead of total number of multiplicands.
// fn test_shared_reference() {
//     let mut rng = test_rng();
//     let ml_extensions: Vec<_> = (0..5)
//         .map(|_| Arc::new(MultiLinearPoly::<Fr>::random(8, &mut rng)))
//         .collect();
//     let mut poly = VirtualPolynomial::new(8);
//     poly.add_mle_list(
//         vec![
//             ml_extensions[2].clone(),
//             ml_extensions[3].clone(),
//             ml_extensions[0].clone(),
//         ],
//         Fr::random_unsafe(&mut rng),
//     );
//     poly.add_mle_list(
//         vec![
//             ml_extensions[1].clone(),
//             ml_extensions[4].clone(),
//             ml_extensions[4].clone(),
//         ],
//         Fr::random_unsafe(&mut rng),
//     );
//     poly.add_mle_list(
//         vec![
//             ml_extensions[3].clone(),
//             ml_extensions[2].clone(),
//             ml_extensions[1].clone(),
//         ],
//         Fr::random_unsafe(&mut rng),
//     );
//     poly.add_mle_list(
//         vec![ml_extensions[0].clone(), ml_extensions[0].clone()],
//         Fr::random_unsafe(&mut rng),
//     );
//     poly.add_mle_list(vec![ml_extensions[4].clone()], Fr::random_unsafe(&mut rng));

//     assert_eq!(poly.flattened_ml_extensions.len(), 5);

//     // test memory usage for prover
//     let prover = ProverState::<Fr>::prover_init(&poly);
//     assert_eq!(prover.virtual_poly.flattened_ml_extensions.len(), 5);
//     drop(prover);

//     let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
//     let poly_info = poly.aux_info.clone();
//     let proof = GenericSumcheck::<Fr>::prove(&poly, &mut transcript);
//     let asserted_sum = GenericSumcheck::<Fr>::extract_sum(&proof);

//     let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
//     let subclaim = GenericSumcheck::<Fr>::verify(asserted_sum, &proof, &poly_info, &mut transcript);
//     assert!(
//         poly.evaluate(&subclaim.point) == subclaim.expected_evaluation,
//         "wrong subclaim"
//     );
// }
