use super::*;
use arith::Fr;
use ark_std::test_rng;
use gkr_hashers::Keccak256hasher;
use polynomials::MultilinearExtension;
use transcript::BytesHashTranscript;

#[test]
fn test_sumcheck() {
    for num_vars in 1..10 {
        let size = 1 << num_vars;

        let coeffs = (1..=size).map(|i| Fr::from(i as u32)).collect::<Vec<_>>();
        let mle = MultiLinearPoly::<Fr>::new(coeffs);
        let asserted_sum = mle.coeffs.iter().sum::<Fr>();

        let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();

        let mut prover = IOPProverState::prover_init(mle.clone());
        let mut verifier = IOPVerifierState::verifier_init(prover.mle.num_vars());

        let mut challenge = None;

        for _ in 0..prover.mle.num_vars() {
            let prover_msg = IOPProverState::prove_round_and_update_state(&mut prover, &challenge);
            // let challenge = IOPVerifierState::verify_round_and_update_state(&mut verifeir,
            // &prover_msg, &mut transcript); transcript.append_serializable_data(&prover_msg);

            challenge = Some(IOPVerifierState::verify_round_and_update_state(
                &mut verifier,
                &prover_msg,
                &mut transcript,
            ));
        }

        println!("prover challenges: {:?}", prover.challenges);
        println!("verifier challenges: {:?}", verifier.challenges);

        let subclaim = IOPVerifierState::check_and_generate_subclaim(&verifier, &asserted_sum);
        assert!(
            mle.eval_reverse_order(&subclaim.point) == subclaim.expected_evaluation,
            "wrong subclaim {:?}   {:?}  ",
            mle.eval_reverse_order(&subclaim.point),
            subclaim.expected_evaluation,
        );
    }
}

// fn test_sumcheck(nv: usize, num_multiplicands_range: (usize, usize), num_products: usize) {
//     let mut rng = test_rng();
//     let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();

//     let (poly, asserted_sum) =
//         VirtualPolynomial::rand(nv, num_multiplicands_range, num_products, &mut rng);

//     println!("poly: {:?}", poly);

//     let proof = Sumcheck::<Fr>::prove(&poly, &mut transcript);
//     let poly_info = poly.aux_info.clone();

//     let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
//     let subclaim = Sumcheck::<Fr>::verify(asserted_sum, &proof, &poly_info, &mut transcript);
//     assert!(
//         poly.evaluate(&subclaim.point) == subclaim.expected_evaluation,
//         "wrong subclaim"
//     );
// }

// fn test_sumcheck_internal(nv: usize, num_multiplicands_range: (usize, usize), num_products:
// usize) {     let mut rng = test_rng();
//     let (poly, asserted_sum) =
//         VirtualPolynomial::<Fr>::rand(nv, num_multiplicands_range, num_products, &mut rng);
//     let poly_info = poly.aux_info.clone();
//     let mut prover_state = IOPProverState::prover_init(&poly);
//     let mut verifier_state = IOPVerifierState::verifier_init(&poly_info);
//     let mut challenge = None;
//     let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
//     // transcript
//     //     .append_message(b"testing", b"initializing transcript for testing")
//     //     .unwrap();
//     for _ in 0..poly.aux_info.num_variables {
//         let prover_message =
//             IOPProverState::prove_round_and_update_state(&mut prover_state, &challenge);

//         challenge = Some(IOPVerifierState::verify_round_and_update_state(
//             &mut verifier_state,
//             &prover_message,
//             &mut transcript,
//         ));
//     }
//     let subclaim = IOPVerifierState::check_and_generate_subclaim(&verifier_state, &asserted_sum);
//     assert!(
//         poly.evaluate(&subclaim.point) == subclaim.expected_evaluation,
//         "wrong subclaim"
//     );
// }

// #[test]
// fn test_trivial_polynomial() {
//     let nv = 1;
//     let num_multiplicands_range = (4, 13);
//     let num_products = 5;

//     test_sumcheck(nv, num_multiplicands_range, num_products);
//     test_sumcheck_internal(nv, num_multiplicands_range, num_products);
// }
// #[test]
// fn test_normal_polynomial() {
//     let nv = 5;
//     let num_multiplicands_range = (1,2);
//     let num_products = 1;

//     test_sumcheck(nv, num_multiplicands_range, num_products);
//     // test_sumcheck_internal(nv, num_multiplicands_range, num_products);
// }
// // #[test]
// // fn zero_polynomial_should_error() {
// //     let nv = 0;
// //     let num_multiplicands_range = (4, 13);
// //     let num_products = 5;

// //     assert!(test_sumcheck(nv, num_multiplicands_range, num_products).is_err());
// //     assert!(test_sumcheck_internal(nv, num_multiplicands_range, num_products).is_err());
// // }

// #[test]
// fn test_extract_sum() {
//     let mut rng = test_rng();
//     let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
//     let (poly, asserted_sum) = VirtualPolynomial::<Fr>::rand(8, (3, 4), 3, &mut rng);

//     let proof = Sumcheck::<Fr>::prove(&poly, &mut transcript);
//     assert_eq!(Sumcheck::<Fr>::extract_sum(&proof), asserted_sum);
// }

// // #[test]
// // /// Test that the memory usage of shared-reference is linear to number of
// // /// unique MLExtensions instead of total number of multiplicands.
// // fn test_shared_reference() -> Result<(), PolyIOPErrors> {
// //     let mut rng = test_rng();
// //     let ml_extensions: Vec<_> = (0..5)
// //         .map(|_| Rc::new(DenseMultilinearExtension::<Fr>::rand(8, &mut rng)))
// //         .collect();
// //     let mut poly = VirtualPolynomial::new(8);
// //     poly.add_mle_list(
// //         vec![
// //             ml_extensions[2].clone(),
// //             ml_extensions[3].clone(),
// //             ml_extensions[0].clone(),
// //         ],
// //         Fr::rand(&mut rng),
// //     )?;
// //     poly.add_mle_list(
// //         vec![
// //             ml_extensions[1].clone(),
// //             ml_extensions[4].clone(),
// //             ml_extensions[4].clone(),
// //         ],
// //         Fr::rand(&mut rng),
// //     )?;
// //     poly.add_mle_list(
// //         vec![
// //             ml_extensions[3].clone(),
// //             ml_extensions[2].clone(),
// //             ml_extensions[1].clone(),
// //         ],
// //         Fr::rand(&mut rng),
// //     )?;
// //     poly.add_mle_list(
// //         vec![ml_extensions[0].clone(), ml_extensions[0].clone()],
// //         Fr::rand(&mut rng),
// //     )?;
// //     poly.add_mle_list(vec![ml_extensions[4].clone()], Fr::rand(&mut rng))?;

// //     assert_eq!(poly.flattened_ml_extensions.len(), 5);

// //     // test memory usage for prover
// //     let prover = IOPProverState::prover_init(&poly).unwrap();
// //     assert_eq!(prover.poly.flattened_ml_extensions.len(), 5);
// //     drop(prover);

// //     let mut transcript = <PolyIOP<Fr> as SumCheck<Fr>>::init_transcript();
// //     let poly_info = poly.aux_info.clone();
// //     let proof = <PolyIOP<Fr> as SumCheck<Fr>>::prove(&poly, &mut transcript)?;
// //     let asserted_sum = <PolyIOP<Fr> as SumCheck<Fr>>::extract_sum(&proof);

// //     let mut transcript = <PolyIOP<Fr> as SumCheck<Fr>>::init_transcript();
// //     let subclaim =
// //         <PolyIOP<Fr> as SumCheck<Fr>>::verify(asserted_sum, &proof, &poly_info, &mut
// // transcript)?;     assert!(
// //         poly.evaluate(&subclaim.point)? == subclaim.expected_evaluation,
// //         "wrong subclaim"
// //     );
// //     Ok(())
// // }
