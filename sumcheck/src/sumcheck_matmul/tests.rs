use super::*;
use arith::Fr;
use gkr_hashers::Keccak256hasher;
use transcript::BytesHashTranscript;

#[test]
fn test_basic_matrix_multiplication() {
    let a_coeffs = vec![
        Fr::from(1u32),
        Fr::from(2u32),
        Fr::from(3u32),
        Fr::from(4u32),
    ];
    let b_coeffs = vec![
        Fr::from(5u32),
        Fr::from(6u32),
        Fr::from(7u32),
        Fr::from(8u32),
    ];

    let mat_a = MatRef {
        coeffs: &a_coeffs,
        rows: 2,
        cols: 2,
    };

    let mat_b = MatRef {
        coeffs: &b_coeffs,
        rows: 2,
        cols: 2,
    };

    let result = mat_a.mat_mul(mat_b);

    // Expected result for 2x2 matrices:
    // [1 2]   [5 6]   [1*5 + 2*7  1*6 + 2*8]   [19 22]
    // [3 4] × [7 8] = [3*5 + 4*7  3*6 + 4*8] = [43 50]
    assert_eq!(result[0], Fr::from(19u32));
    assert_eq!(result[1], Fr::from(22u32));
    assert_eq!(result[2], Fr::from(43u32));
    assert_eq!(result[3], Fr::from(50u32));
}

#[test]
fn test_rectangular_matrix_multiplication() {
    let a_coeffs = vec![
        Fr::from(1u32),
        Fr::from(2u32),
        Fr::from(3u32),
        Fr::from(4u32),
        Fr::from(5u32),
        Fr::from(6u32),
    ];
    let b_coeffs = vec![
        Fr::from(7u32),
        Fr::from(8u32),
        Fr::from(9u32),
        Fr::from(10u32),
        Fr::from(11u32),
        Fr::from(12u32),
    ];

    let mat_a = MatRef {
        coeffs: &a_coeffs,
        rows: 2,
        cols: 3,
    };

    let mat_b = MatRef {
        coeffs: &b_coeffs,
        rows: 3,
        cols: 2,
    };

    let result = mat_a.mat_mul(mat_b);

    // Expected result for 2x3 × 3x2 matrices:
    // [1 2 3]   [7  8 ]   [1*7 + 2*9 + 3*11  1*8 + 2*10 + 3*12]   [58  64]
    // [4 5 6] × [9  10] = [4*7 + 5*9 + 6*11  4*8 + 5*10 + 6*12] = [139 154]
    //           [11 12]
    assert_eq!(result[0], Fr::from(58u32));
    assert_eq!(result[1], Fr::from(64u32));
    assert_eq!(result[2], Fr::from(139u32));
    assert_eq!(result[3], Fr::from(154u32));
}

#[test]
#[should_panic(expected = "Matrix dimensions must match for multiplication")]
fn test_invalid_dimensions() {
    let a_coeffs = vec![
        Fr::from(1u32),
        Fr::from(2u32),
        Fr::from(3u32),
        Fr::from(4u32),
    ];
    let b_coeffs = vec![
        Fr::from(5u32),
        Fr::from(6u32),
        Fr::from(7u32),
        Fr::from(8u32),
        Fr::from(9u32),
        Fr::from(10u32),
    ];

    let mat_a = MatRef {
        coeffs: &a_coeffs,
        rows: 2,
        cols: 2,
    };

    let mat_b = MatRef {
        coeffs: &b_coeffs,
        rows: 3,
        cols: 2,
    };

    let _ = mat_a.mat_mul(mat_b); // This should panic
}

#[test]
fn test_zero_matrix_multiplication() {
    let a_coeffs = vec![Fr::zero(); 4];
    let b_coeffs = vec![
        Fr::from(1u32),
        Fr::from(2u32),
        Fr::from(3u32),
        Fr::from(4u32),
    ];

    let mat_a = MatRef {
        coeffs: &a_coeffs,
        rows: 2,
        cols: 2,
    };

    let mat_b = MatRef {
        coeffs: &b_coeffs,
        rows: 2,
        cols: 2,
    };

    let result = mat_a.mat_mul(mat_b);

    // Multiplying by zero matrix should give zero matrix
    assert_eq!(result[0], Fr::zero());
    assert_eq!(result[1], Fr::zero());
    assert_eq!(result[2], Fr::zero());
    assert_eq!(result[3], Fr::zero());
}

#[test]
fn test_from_mle_via_rlc_2x2() {
    let coeffs = vec![
        Fr::from(1u32),
        Fr::from(2u32),
        Fr::from(3u32),
        Fr::from(4u32),
    ];

    let mat = MatRef {
        coeffs: &coeffs,
        rows: 2,
        cols: 2,
    };

    let r = Fr::from(2u32);
    let result = mat.from_mle_via_rlc(&r);

    // For r = 2, powers_of_r = [1, 2]
    // First column: 1*1 + 2*3 = 7
    // Second column: 1*2 + 2*4 = 10
    assert_eq!(result[0], Fr::from(7u32));
    assert_eq!(result[1], Fr::from(10u32));
}

#[test]
fn test_from_mle_via_rlc_2x3() {
    let coeffs = vec![
        Fr::from(1u32),
        Fr::from(2u32),
        Fr::from(3u32),
        Fr::from(4u32),
        Fr::from(5u32),
        Fr::from(6u32),
    ];

    let mat = MatRef {
        coeffs: &coeffs,
        rows: 2,
        cols: 3,
    };

    let r = Fr::from(3u32);
    let result = mat.from_mle_via_rlc(&r);

    // For r = 3, powers_of_r = [1, 3]
    // First column: 1*1 + 3*4 = 13
    // Second column: 1*2 + 3*5 = 17
    // Third column: 1*3 + 3*6 = 21
    assert_eq!(result[0], Fr::from(13u32));
    assert_eq!(result[1], Fr::from(17u32));
    assert_eq!(result[2], Fr::from(21u32));
}

#[test]
fn test_from_mle_via_rlc_zero_matrix() {
    let coeffs = vec![Fr::zero(); 4];

    let mat = MatRef {
        coeffs: &coeffs,
        rows: 2,
        cols: 2,
    };

    let r = Fr::from(5u32);
    let result = mat.from_mle_via_rlc(&r);

    // Any linear combination of zero matrix should give zero vector
    assert_eq!(result[0], Fr::zero());
    assert_eq!(result[1], Fr::zero());
}

#[test]
fn test_from_mle_via_rlc_identity_matrix() {
    let coeffs = vec![
        Fr::from(1u32),
        Fr::from(0u32),
        Fr::from(0u32),
        Fr::from(1u32),
    ];

    let mat = MatRef {
        coeffs: &coeffs,
        rows: 2,
        cols: 2,
    };

    let r = Fr::from(2u32);
    let result = mat.from_mle_via_rlc(&r);

    // For identity matrix with r = 2, powers_of_r = [1, 2]
    // First column: 1*1 + 2*0 = 1
    // Second column: 1*0 + 2*1 = 2
    assert_eq!(result[0], Fr::from(1u32));
    assert_eq!(result[1], Fr::from(2u32));
}

#[test]
fn test_transpose_square_matrix() {
    // 2x2 matrix: [[1, 2], [3, 4]]
    let coeffs = vec![
        Fr::from(1u32),
        Fr::from(2u32),
        Fr::from(3u32),
        Fr::from(4u32),
    ];
    let mat_ref = MatRef {
        coeffs: &coeffs,
        rows: 2,
        cols: 2,
    };

    let transposed = mat_ref.transpose();

    // Expected: [[1, 3], [2, 4]]
    let expected = vec![
        Fr::from(1u32),
        Fr::from(3u32),
        Fr::from(2u32),
        Fr::from(4u32),
    ];

    assert_eq!(transposed.rows, 2);
    assert_eq!(transposed.cols, 2);
    assert_eq!(transposed.coeffs, expected);
}

#[test]
fn test_transpose_rectangular_matrix() {
    // 2x3 matrix: [[1, 2, 3], [4, 5, 6]]
    let coeffs = vec![
        Fr::from(1u32),
        Fr::from(2u32),
        Fr::from(3u32),
        Fr::from(4u32),
        Fr::from(5u32),
        Fr::from(6u32),
    ];
    let mat_ref = MatRef {
        coeffs: &coeffs,
        rows: 2,
        cols: 3,
    };

    let transposed = mat_ref.transpose();

    // Expected 3x2 matrix: [[1, 4], [2, 5], [3, 6]]
    let expected = vec![
        Fr::from(1u32),
        Fr::from(4u32),
        Fr::from(2u32),
        Fr::from(5u32),
        Fr::from(3u32),
        Fr::from(6u32),
    ];

    assert_eq!(transposed.rows, 3);
    assert_eq!(transposed.cols, 2);
    assert_eq!(transposed.coeffs, expected);
}

#[test]
fn test_transpose_single_row() {
    // 1x3 matrix: [[1, 2, 3]]
    let coeffs = vec![Fr::from(1u32), Fr::from(2u32), Fr::from(3u32)];
    let mat_ref = MatRef {
        coeffs: &coeffs,
        rows: 1,
        cols: 3,
    };

    let transposed = mat_ref.transpose();

    // Expected 3x1 matrix: [[1], [2], [3]]
    let expected = vec![Fr::from(1u32), Fr::from(2u32), Fr::from(3u32)];

    assert_eq!(transposed.rows, 3);
    assert_eq!(transposed.cols, 1);
    assert_eq!(transposed.coeffs, expected);
}

#[test]
fn test_transpose_single_column() {
    // 3x1 matrix: [[1], [2], [3]]
    let coeffs = vec![Fr::from(1u32), Fr::from(2u32), Fr::from(3u32)];
    let mat_ref = MatRef {
        coeffs: &coeffs,
        rows: 3,
        cols: 1,
    };

    let transposed = mat_ref.transpose();

    // Expected 1x3 matrix: [[1, 2, 3]]
    let expected = vec![Fr::from(1u32), Fr::from(2u32), Fr::from(3u32)];

    assert_eq!(transposed.rows, 1);
    assert_eq!(transposed.cols, 3);
    assert_eq!(transposed.coeffs, expected);
}

#[test]
fn test_double_transpose() {
    // Double transpose should return original matrix
    let coeffs = vec![
        Fr::from(1u32),
        Fr::from(2u32),
        Fr::from(3u32),
        Fr::from(4u32),
        Fr::from(5u32),
        Fr::from(6u32),
    ];
    let mat_ref = MatRef {
        coeffs: &coeffs,
        rows: 2,
        cols: 3,
    };

    let transposed = mat_ref.transpose();
    let transposed_ref = MatRef {
        coeffs: &transposed.coeffs,
        rows: transposed.rows,
        cols: transposed.cols,
    };
    let double_transposed = transposed_ref.transpose();

    assert_eq!(double_transposed.rows, mat_ref.rows);
    assert_eq!(double_transposed.cols, mat_ref.cols);
    assert_eq!(double_transposed.coeffs, coeffs);
}

#[test]
fn test_transpose_with_zeros() {
    // Matrix with zero elements
    let coeffs = vec![
        Fr::from(0u32),
        Fr::from(1u32),
        Fr::from(0u32),
        Fr::from(2u32),
    ];
    let mat_ref = MatRef {
        coeffs: &coeffs,
        rows: 2,
        cols: 2,
    };

    let transposed = mat_ref.transpose();
    let expected = vec![
        Fr::from(0u32),
        Fr::from(0u32),
        Fr::from(1u32),
        Fr::from(2u32),
    ];

    assert_eq!(transposed.coeffs, expected);
}

#[test]
fn test_transpose_then_multiply() {
    // Test that transpose works correctly with matrix multiplication
    // Create a 2x3 matrix
    let a_coeffs = vec![
        Fr::from(1u32),
        Fr::from(2u32),
        Fr::from(3u32),
        Fr::from(4u32),
        Fr::from(5u32),
        Fr::from(6u32),
    ];
    let mat_a = MatRef {
        coeffs: &a_coeffs,
        rows: 2,
        cols: 3,
    };

    // Transpose to get 3x2 matrix
    let transposed = mat_a.transpose();
    let transposed_ref = MatRef {
        coeffs: &transposed.coeffs,
        rows: transposed.rows,
        cols: transposed.cols,
    };

    // Now multiply original (2x3) by transposed (3x2) to get 2x2
    let result = mat_a.mat_mul(transposed_ref);

    // Expected: A * A^T where A = [[1,2,3], [4,5,6]]
    // [[1,2,3], [4,5,6]] * [[1,4], [2,5], [3,6]] = [[14,32], [32,77]]
    assert_eq!(result.len(), 4); // 2x2 result
    assert_eq!(result[0], Fr::from(14u32)); // 1*1 + 2*2 + 3*3
    assert_eq!(result[1], Fr::from(32u32)); // 1*4 + 2*5 + 3*6
    assert_eq!(result[2], Fr::from(32u32)); // 4*1 + 5*2 + 6*3
    assert_eq!(result[3], Fr::from(77u32)); // 4*4 + 5*5 + 6*6
}

#[test]
fn test_sumcheck_matmul_basic_2x2() {
    // Test basic 2x2 matrix multiplication verification
    let a_coeffs = vec![
        Fr::from(1u32),
        Fr::from(2u32),
        Fr::from(3u32),
        Fr::from(4u32),
    ];
    let b_coeffs = vec![
        Fr::from(5u32),
        Fr::from(6u32),
        Fr::from(7u32),
        Fr::from(8u32),
    ];

    let mat_a = MatRef {
        coeffs: &a_coeffs,
        rows: 2,
        cols: 2,
    };
    let mat_b = MatRef {
        coeffs: &b_coeffs,
        rows: 2,
        cols: 2,
    };

    // Compute expected result: A * B
    let expected_result = mat_a.mat_mul(mat_b);
    let mat_c = MatRef {
        coeffs: &expected_result,
        rows: 2,
        cols: 2,
    };

    let witnesses = MatMulWitnesses::new(mat_a, mat_b, mat_c);

    let mut prover_transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let mut verifier_transcript = BytesHashTranscript::<Keccak256hasher>::new();

    // Generate proof
    let proof = SumCheckMatMul::prove(&witnesses, &mut prover_transcript);

    // Extract claimed sum
    let claimed_sum = SumCheckMatMul::extract_sum(&proof);

    // Verify the proof
    let num_vars = 1;
    let (verified, subclaim) = SumCheckMatMul::verify(&proof, &mut verifier_transcript);
    assert!(
        verified,
        "Verification failed for basic 2x2 matrix multiplication"
    );

    println!("subclaim: {:?}", subclaim);

    // The sumcheck should succeed (subclaim should be valid)
    let mut test_transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let (mle_list, _) = witnesses.form_zerocheck_polynomial(&mut test_transcript);
    let evals = mle_list.evaluate(&subclaim.point);
    assert!(evals == subclaim.expected_evaluation, "wrong subclaim");
}

#[test]
fn test_sumcheck_matmul_rectangular_matrices() {
    // Test with 2x3 * 3x2 matrices
    let a_coeffs = vec![
        Fr::from(1u32),
        Fr::from(2u32),
        Fr::from(3u32),
        Fr::from(4u32),
        Fr::from(5u32),
        Fr::from(6u32),
    ];
    let b_coeffs = vec![
        Fr::from(7u32),
        Fr::from(8u32),
        Fr::from(9u32),
        Fr::from(10u32),
        Fr::from(11u32),
        Fr::from(12u32),
    ];

    let mat_a = MatRef {
        coeffs: &a_coeffs,
        rows: 2,
        cols: 3,
    };
    let mat_b = MatRef {
        coeffs: &b_coeffs,
        rows: 3,
        cols: 2,
    };

    let expected_result = mat_a.mat_mul(mat_b);
    let mat_c = MatRef {
        coeffs: &expected_result,
        rows: 2,
        cols: 2,
    };

    let witnesses = MatMulWitnesses::new(mat_a, mat_b, mat_c);

    let mut prover_transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let mut verifier_transcript = BytesHashTranscript::<Keccak256hasher>::new();

    let proof = SumCheckMatMul::prove(&witnesses, &mut prover_transcript);
    let claimed_sum = SumCheckMatMul::extract_sum(&proof);

    let num_vars = 2; // Depends on your specific implementation
    let (verified, subclaim) = SumCheckMatMul::verify(&proof, &mut verifier_transcript);
    assert!(verified, "Verification failed for rectangular matrices");

    let mut test_transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let (mle_list, _) = witnesses.form_zerocheck_polynomial(&mut test_transcript);
    let evals = mle_list.evaluate(&subclaim.point);
    assert!(evals == subclaim.expected_evaluation, "wrong subclaim");
}
