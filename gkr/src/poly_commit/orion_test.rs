use std::marker::PhantomData;

use arith::{ExtensionField, Field, FieldSerde, SimdField};
use ark_std::{log2, test_rng};
use gf2::{GF2x64, GF2x8, GF2};
use gf2_128::GF2_128;
use mersenne31::{M31Ext3, M31x16, M31};
use polynomials::{EqPolynomial, MultiLinearPoly};
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};
use tree::{Leaf, Tree};

use crate::{transpose_in_place, OrionCode, OrionCodeParameter, ORION_PCS_SOUNDNESS_BITS};

use super::{OrionCommitmentWithData, OrionPCSImpl};

fn column_combination<F: Field>(mat: &[F], combination: &[F]) -> Vec<F> {
    mat.chunks(combination.len())
        .map(|row_i| {
            row_i
                .iter()
                .zip(combination.iter())
                .map(|(&r_ij, &c_j)| r_ij * c_j)
                .sum()
        })
        .collect()
}

// NOTE: beware - this is a sketch code parameter from
// https://eprint.iacr.org/2022/1010.pdf (Orion) p8
// on general Spielman code.
// This set of params might not be carefully calculated for soundness.
// Only used here for testing purpose
const EXAMPLE_ORION_CODE_PARAMETER: OrionCodeParameter = OrionCodeParameter {
    input_message_len: 1 << 10,
    output_code_len: 1 << 12,

    alpha_g0: 0.5,
    degree_g0: 6,

    lenghth_threshold_g0s: 10,

    degree_g1: 6,

    // TODO: update to real parameter
    hamming_weight: 0.07,
};

fn test_orion_code_generic<F: Field>() {
    let mut rng = test_rng();

    let orion_code = OrionCode::new(EXAMPLE_ORION_CODE_PARAMETER, &mut rng);

    let linear_combine_size = 128;

    let random_scalrs: Vec<_> = (0..linear_combine_size)
        .map(|_| F::random_unsafe(&mut rng))
        .collect();

    // NOTE: generate message and codeword in the slice buffer

    let mut message_mat =
        vec![F::ZERO; linear_combine_size * EXAMPLE_ORION_CODE_PARAMETER.input_message_len];

    let mut codeword_mat =
        vec![F::ZERO; linear_combine_size * EXAMPLE_ORION_CODE_PARAMETER.output_code_len];

    message_mat
        .chunks_mut(EXAMPLE_ORION_CODE_PARAMETER.input_message_len)
        .zip(codeword_mat.chunks_mut(EXAMPLE_ORION_CODE_PARAMETER.output_code_len))
        .try_for_each(|(msg, codeword)| {
            msg.iter_mut().for_each(|x| *x = F::random_unsafe(&mut rng));
            orion_code.encode_in_place(msg, codeword)
        })
        .unwrap();

    // NOTE: transpose message and codeword matrix

    let mut message_scratch =
        vec![F::ZERO; linear_combine_size * EXAMPLE_ORION_CODE_PARAMETER.input_message_len];
    transpose_in_place(&mut message_mat, &mut message_scratch, linear_combine_size);
    drop(message_scratch);

    let mut codeword_scratch =
        vec![F::ZERO; linear_combine_size * EXAMPLE_ORION_CODE_PARAMETER.output_code_len];
    transpose_in_place(
        &mut codeword_mat,
        &mut codeword_scratch,
        linear_combine_size,
    );
    drop(codeword_scratch);

    // NOTE: message and codeword matrix linear combination with weights

    let msg_linear_combined = column_combination(&message_mat, &random_scalrs);
    let codeword_linear_combined = column_combination(&codeword_mat, &random_scalrs);

    let codeword_computed = orion_code.encode(&msg_linear_combined).unwrap();

    assert_eq!(codeword_linear_combined, codeword_computed);
}

#[test]
fn test_orion_code() {
    test_orion_code_generic::<GF2_128>();
    test_orion_code_generic::<GF2>();
    test_orion_code_generic::<M31Ext3>();
}

impl OrionPCSImpl {
    fn dumb_commit<F: Field + FieldSerde, PackF: SimdField<Scalar = F>>(
        &self,
        poly: &MultiLinearPoly<F>,
    ) -> OrionCommitmentWithData<F, PackF> {
        let (row_num, msg_size) = Self::row_col_from_variables(poly.get_num_vars());

        let mut interleaved_codewords: Vec<_> = poly
            .coeffs
            .chunks(msg_size)
            .flat_map(|msg| self.code_instance.encode(&msg).unwrap())
            .collect();

        let mut scratch = vec![F::ZERO; row_num * self.code_len()];
        transpose_in_place(&mut interleaved_codewords, &mut scratch, row_num);
        drop(scratch);

        OrionCommitmentWithData {
            num_of_variables: poly.get_num_vars(),
            interleaved_codewords,

            interleaved_alphabet_trees: Vec::new(),
            commitment_tree: Tree::new_with_leaves(vec![Leaf::default(), Leaf::default()]),

            _phantom: PhantomData,
        }
    }
}

fn test_orion_commit_consistency_generic<F: Field + FieldSerde, PackF: SimdField<Scalar = F>>() {
    let mut rng = test_rng();
    let num_of_vars = log2(EXAMPLE_ORION_CODE_PARAMETER.input_message_len) as usize * 2usize;

    let random_poly = MultiLinearPoly::<F>::random(num_of_vars, &mut rng);

    let orion_pcs =
        OrionPCSImpl::from_random(num_of_vars, EXAMPLE_ORION_CODE_PARAMETER, &mut rng).unwrap();

    let real_commit = orion_pcs.commit::<F, PackF>(&random_poly).unwrap();
    let dumb_commit = orion_pcs.dumb_commit::<F, PackF>(&random_poly);

    assert_eq!(real_commit.num_of_variables, dumb_commit.num_of_variables);
    assert_eq!(
        real_commit.interleaved_codewords,
        dumb_commit.interleaved_codewords
    );
}

#[test]
fn test_orion_commit_consistency() {
    test_orion_commit_consistency_generic::<GF2, GF2x8>();
    test_orion_commit_consistency_generic::<GF2, GF2x64>();
    test_orion_commit_consistency_generic::<GF2, GF2_128>();
    test_orion_commit_consistency_generic::<M31, M31x16>();
}

fn test_multilinear_poly_tensor_eval_generic<F: Field, ExtF: ExtensionField<BaseField = F>>(
    num_of_vars: usize,
) {
    let mut rng = test_rng();

    let random_poly = MultiLinearPoly::<F>::random(num_of_vars, &mut rng);
    let random_poly_ext =
        MultiLinearPoly::new(random_poly.coeffs.iter().cloned().map(ExtF::from).collect());

    let random_point: Vec<_> = (0..num_of_vars)
        .map(|_| ExtF::random_unsafe(&mut rng))
        .collect();

    let expected_eval = random_poly_ext.evaluate_jolt(&random_point);

    let (_row_num, col_num) = OrionPCSImpl::row_col_from_variables(num_of_vars);
    // row for higher vars, cols for lower vars
    let vars_for_col = log2(col_num) as usize;

    let mut random_poly_ext_half_evaluated = random_poly_ext.clone();
    random_point[vars_for_col..]
        .iter()
        .rev()
        .for_each(|p| random_poly_ext_half_evaluated.fix_top_variable(p));

    let eq_linear_combination = EqPolynomial::build_eq_x_r(&random_point[..vars_for_col]);
    let actual_eval: ExtF = random_poly_ext_half_evaluated
        .coeffs
        .iter()
        .zip(eq_linear_combination.iter())
        .map(|(&c, &eq_c)| c * eq_c)
        .sum();

    assert_eq!(expected_eval, actual_eval)
}

#[test]
fn test_multilinear_poly_tensor_eval() {
    (10..22).for_each(|vars| {
        test_multilinear_poly_tensor_eval_generic::<GF2, GF2_128>(vars);
        test_multilinear_poly_tensor_eval_generic::<M31, M31Ext3>(vars)
    })
}

fn test_orion_pcs_open_generics<
    F: Field + FieldSerde,
    ExtF: ExtensionField<BaseField = F>,
    PackF: SimdField<Scalar = F>,
>() {
    let mut rng = test_rng();
    let num_of_vars = log2(EXAMPLE_ORION_CODE_PARAMETER.input_message_len) as usize * 2usize;

    let random_poly = MultiLinearPoly::<F>::random(num_of_vars, &mut rng);
    let random_poly_ext =
        MultiLinearPoly::new(random_poly.coeffs.iter().cloned().map(ExtF::from).collect());
    let random_point: Vec<_> = (0..num_of_vars)
        .map(|_| ExtF::random_bool(&mut rng))
        .collect();

    let mut transcript: BytesHashTranscript<ExtF, Keccak256hasher> = BytesHashTranscript::new();
    let mut transcript_cloned = transcript.clone();

    let orion_pcs =
        OrionPCSImpl::from_random(num_of_vars, EXAMPLE_ORION_CODE_PARAMETER, &mut rng).unwrap();

    let commit_with_data = orion_pcs.commit::<F, PackF>(&random_poly).unwrap();

    let opening = orion_pcs.open(
        &random_poly,
        &commit_with_data,
        &random_point,
        &mut transcript,
    );

    // NOTE: evaluation consistency check
    let (row_num, col_num) = OrionPCSImpl::row_col_from_variables(num_of_vars);
    let vars_for_col = log2(col_num) as usize;
    let poly_half_evaled = MultiLinearPoly::new(opening.eval_row.clone());
    let actual_eval = poly_half_evaled.evaluate_jolt(&random_point[..vars_for_col]);
    let expected_eval = random_poly_ext.evaluate_jolt(&random_point);
    assert_eq!(expected_eval, actual_eval);

    // NOTE: compute evaluation codeword
    let eval_codeword = orion_pcs.code_instance.encode(&opening.eval_row).unwrap();
    let eq_linear_combination = EqPolynomial::build_eq_x_r(&random_point[vars_for_col..]);
    let interleaved_codeword_ext = commit_with_data
        .interleaved_codewords
        .iter()
        .cloned()
        .map(ExtF::from)
        .collect::<Vec<_>>();

    let eq_combined_codeword =
        column_combination(&interleaved_codeword_ext, &eq_linear_combination);
    assert_eq!(eval_codeword, eq_combined_codeword);

    // NOTE: compute proximity codewords
    let proximity_repetitions =
        orion_pcs.proximity_repetition_num(ORION_PCS_SOUNDNESS_BITS, ExtF::FIELD_SIZE);
    assert_eq!(proximity_repetitions, opening.proximity_rows.len());

    opening.proximity_rows.iter().for_each(|proximity_row| {
        let random_linear_combination =
            transcript_cloned.generate_challenge_field_elements(row_num);

        let expected_proximity_codeword =
            column_combination(&interleaved_codeword_ext, &random_linear_combination);

        let actual_proximity_codeword = orion_pcs.code_instance.encode(proximity_row).unwrap();

        assert_eq!(expected_proximity_codeword, actual_proximity_codeword)
    });
}

#[test]
fn test_orion_pcs_open() {
    test_orion_pcs_open_generics::<GF2, GF2_128, GF2_128>();
    test_orion_pcs_open_generics::<M31, M31Ext3, M31x16>()
}

fn test_orion_pcs_full_e2e_generics<
    F: Field + FieldSerde,
    ExtF: ExtensionField<BaseField = F>,
    PackF: SimdField<Scalar = F>,
>() {
    let mut rng = test_rng();
    let num_of_vars = log2(EXAMPLE_ORION_CODE_PARAMETER.input_message_len) as usize * 2usize;

    let random_poly = MultiLinearPoly::<F>::random(num_of_vars, &mut rng);
    let random_poly_ext =
        MultiLinearPoly::new(random_poly.coeffs.iter().cloned().map(ExtF::from).collect());
    let random_point: Vec<_> = (0..num_of_vars)
        .map(|_| ExtF::random_bool(&mut rng))
        .collect();
    let expected_eval = random_poly_ext.evaluate_jolt(&random_point);

    let mut transcript: BytesHashTranscript<ExtF, Keccak256hasher> = BytesHashTranscript::new();
    let mut transcript_cloned = transcript.clone();

    let orion_pcs =
        OrionPCSImpl::from_random(num_of_vars, EXAMPLE_ORION_CODE_PARAMETER, &mut rng).unwrap();

    let commit_with_data = orion_pcs.commit::<F, PackF>(&random_poly).unwrap();

    let opening = orion_pcs.open(
        &random_poly,
        &commit_with_data,
        &random_point,
        &mut transcript,
    );

    assert!(
        orion_pcs.verify::<F, PackF, ExtF, BytesHashTranscript<ExtF, Keccak256hasher>>(
            &commit_with_data.commitment_tree.root(),
            &random_point,
            &expected_eval,
            &opening,
            &mut transcript_cloned
        )
    );
}

#[test]
fn test_orion_pcs_full_e2e() {
    test_orion_pcs_full_e2e_generics::<GF2, GF2_128, GF2x8>();
    test_orion_pcs_full_e2e_generics::<GF2, GF2_128, GF2x64>();
    test_orion_pcs_full_e2e_generics::<GF2, GF2_128, GF2_128>();
    // test_orion_pcs_full_e2e_generics::<M31, M31Ext3, M31x16>()
}