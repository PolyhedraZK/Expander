use arith::Field;
use halo2curves::bn256::Fr;
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};

use crate::prover_helper::SumcheckInstanceProof;

fn setup_test_polynomials<F: Field>(num_vars: usize) -> Vec<MultiLinearPoly<F>> {
    // Create two test polynomials:
    // P1(x1,x2,...,xn) = x1 + x2 + ... + xn
    // P2(x1,x2,...,xn) = x1 * x2 * ... * xn

    let size = 1 << num_vars; // 2^num_vars
    let mut p1_coeffs = Vec::with_capacity(size);
    let mut p2_coeffs = Vec::with_capacity(size);

    for i in 0..size {
        // For P1: coefficient is 1 if exactly one bit is set, 0 otherwise
        p1_coeffs.push(F::from(i.count_ones() as u32));

        // For P2: coefficient is 1 if all bits are set, 0 otherwise
        p2_coeffs.push(if i == size - 1 { F::one() } else { F::zero() });
    }

    vec![
        MultiLinearPoly::new(p1_coeffs),
        MultiLinearPoly::new(p2_coeffs),
    ]
}

fn combination_function<F: Field>(evals: &[F]) -> F {
    // Simple combination function: sum of all evaluations
    evals.iter().fold(F::zero(), |acc, &x| acc + x)
}

#[test]
fn test_sumcheck_basic() {
    let num_vars = 3;
    println!("Setup polynomial");
    let mut polys = setup_test_polynomials(num_vars);
    let polys_copy = polys.clone();
    let mut transcript = BytesHashTranscript::<_, Keccak256hasher>::new();

    // Calculate the actual sum by evaluating at all boolean inputs
    println!("generate sum");
    let mut actual_sum = Fr::zero();
    for i in 0..(1 << num_vars) {
        let point: Vec<Fr> = (0..num_vars)
            .map(|j| {
                if (i >> j) & 1 == 1 {
                    Fr::one()
                } else {
                    Fr::zero()
                }
            })
            .collect();

        let evals: Vec<Fr> = polys.iter().map(|poly| poly.evaluate(&point)).collect();
        actual_sum += combination_function(&evals);
    }

    println!("creat proof");
    // Create the proof
    let combined_degree = 2; // Sum of two linear polynomials has degree 2
    let (proof, r_points, final_evals) = SumcheckInstanceProof::prove_arbitrary(
        &actual_sum,
        num_vars,
        &mut polys,
        combination_function,
        combined_degree,
        &mut transcript,
    );

    // Reset transcript for verification
    println!("verify proof");
    let mut verify_transcript = BytesHashTranscript::<_, Keccak256hasher>::new();

    // Verify the proof
    let (claimed_eval, verify_points) = proof.verify(
        actual_sum,
        num_vars,
        combined_degree,
        &mut verify_transcript,
    );

    println!("check sum");
    // Check that verification points match proof points
    assert_eq!(
        r_points, verify_points,
        "Verification points don't match proof points"
    );

    println!("final check");
    // Evaluate the original polynomials at the final random point
    let final_point: Vec<Fr> = verify_points.clone();
    let expected_evals: Vec<Fr> = polys_copy
        .iter()
        .map(|poly| poly.evaluate(&final_point))
        .collect();

    // Check that the final evaluations match
    assert_eq!(final_evals, expected_evals, "Final evaluations don't match");

    // Check that combining the final evaluations gives the claimed evaluation
    let combined_eval = combination_function(&final_evals);
    assert_eq!(
        claimed_eval, combined_eval,
        "Combined evaluation doesn't match claimed evaluation"
    );
}
