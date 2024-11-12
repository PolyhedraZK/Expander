use std::{marker::PhantomData, ops::Mul};

use arith::{ExtensionField, Field, FieldSerde, SimdField};
use ark_std::{log2, test_rng};
use gf2::{GF2x128, GF2x64, GF2x8, GF2};
use gf2_128::GF2_128;
use mersenne31::{M31Ext3, M31x16, M31};
use polynomials::{EqPolynomial, MultiLinearPoly};
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};

use crate::{
    inner_product, transpose_in_place, OrionCode, OrionCommitment, ORION_CODE_PARAMETER_INSTANCE,
    ORION_PCS_SOUNDNESS_BITS,
};

use super::{OrionCommitmentWithData, OrionPublicParams};

fn column_combination<F: Field>(mat: &[F], combination: &[F]) -> Vec<F> {
    mat.chunks(combination.len())
        .map(|row_i| inner_product(row_i, combination, |a, b| *a * *b))
        .collect()
}

fn test_orion_code_generic<F: Field>(msg_len: usize) {
    let mut rng = test_rng();

    let orion_code = OrionCode::new(ORION_CODE_PARAMETER_INSTANCE, msg_len, &mut rng);

    let linear_combine_size = 128;

    let random_scalrs: Vec<_> = (0..linear_combine_size)
        .map(|_| F::random_unsafe(&mut rng))
        .collect();

    // NOTE: generate message and codeword in the slice buffer

    let mut message_mat = vec![F::ZERO; linear_combine_size * orion_code.msg_len()];

    let mut codeword_mat = vec![F::ZERO; linear_combine_size * orion_code.code_len()];

    message_mat
        .chunks_mut(orion_code.msg_len())
        .zip(codeword_mat.chunks_mut(orion_code.code_len()))
        .try_for_each(|(msg, codeword)| {
            msg.iter_mut().for_each(|x| *x = F::random_unsafe(&mut rng));
            orion_code.encode_in_place(msg, codeword)
        })
        .unwrap();

    // NOTE: transpose message and codeword matrix

    let mut message_scratch = vec![F::ZERO; linear_combine_size * orion_code.msg_len()];
    transpose_in_place(&mut message_mat, &mut message_scratch, linear_combine_size);
    drop(message_scratch);

    let mut codeword_scratch = vec![F::ZERO; linear_combine_size * orion_code.code_len()];
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
    (5..=15).for_each(|num_vars| {
        let msg_len = 1usize << num_vars;

        test_orion_code_generic::<GF2_128>(msg_len);
        test_orion_code_generic::<GF2>(msg_len);
        test_orion_code_generic::<M31Ext3>(msg_len);
    });
}

impl OrionPublicParams {
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

        if !interleaved_codewords.len().is_power_of_two() {
            let aligned_po2_len = interleaved_codewords.len().next_power_of_two();
            interleaved_codewords.resize(aligned_po2_len, F::default());
        }

        let interleaved_alphabet_tree =
            tree::Tree::compact_new_with_field_elems::<F, PackF>(&interleaved_codewords);

        OrionCommitmentWithData {
            interleaved_alphabet_tree,
            _phantom: PhantomData,
        }
    }
}

fn test_orion_commit_consistency_generic<F, PackF>(num_vars: usize)
where
    F: Field + FieldSerde,
    PackF: SimdField<Scalar = F>,
{
    let mut rng = test_rng();

    let random_poly = MultiLinearPoly::<F>::random(num_vars, &mut rng);
    let orion_pcs =
        OrionPublicParams::from_random(num_vars, ORION_CODE_PARAMETER_INSTANCE, &mut rng);

    let real_commit = orion_pcs.commit::<F, PackF>(&random_poly).unwrap();
    let dumb_commit = orion_pcs.dumb_commit::<F, PackF>(&random_poly);

    let real_commitment: OrionCommitment = real_commit.into();
    let dumb_commitment: OrionCommitment = dumb_commit.into();

    assert_eq!(real_commitment, dumb_commitment);
}

#[test]
fn test_orion_commit_consistency() {
    (19..=25).for_each(|num_vars| {
        test_orion_commit_consistency_generic::<GF2, GF2x8>(num_vars);
        test_orion_commit_consistency_generic::<GF2, GF2x64>(num_vars);
        test_orion_commit_consistency_generic::<GF2, GF2x128>(num_vars);
        // test_orion_commit_consistency_generic::<M31, M31x16>(num_vars);
    })
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

    let (_row_num, col_num) = OrionPublicParams::row_col_from_variables(num_of_vars);
    // row for higher vars, cols for lower vars
    let vars_for_col = log2(col_num) as usize;

    let mut random_poly_ext_half_evaluated = random_poly_ext.clone();
    random_point[vars_for_col..]
        .iter()
        .rev()
        .for_each(|p| random_poly_ext_half_evaluated.fix_top_variable(p));

    let eq_linear_combination = EqPolynomial::build_eq_x_r(&random_point[..vars_for_col]);
    let actual_eval: ExtF = inner_product(
        &random_poly_ext_half_evaluated.coeffs,
        &eq_linear_combination,
        |a, b| *a * *b,
    );

    assert_eq!(expected_eval, actual_eval)
}

#[test]
fn test_multilinear_poly_tensor_eval() {
    (10..22).for_each(|vars| {
        test_multilinear_poly_tensor_eval_generic::<GF2, GF2_128>(vars);
        test_multilinear_poly_tensor_eval_generic::<M31, M31Ext3>(vars)
    })
}

fn test_orion_pcs_open_generics<F, EvalF, PackF>(num_vars: usize)
where
    F: Field + FieldSerde,
    EvalF: Field + FieldSerde + From<F> + Mul<F, Output = EvalF>,
    PackF: SimdField<Scalar = F>,
{
    let mut rng = test_rng();

    let random_poly = MultiLinearPoly::<F>::random(num_vars, &mut rng);
    let random_poly_ext = MultiLinearPoly::new(
        random_poly
            .coeffs
            .iter()
            .cloned()
            .map(EvalF::from)
            .collect(),
    );
    let random_point: Vec<_> = (0..num_vars)
        .map(|_| EvalF::random_bool(&mut rng))
        .collect();

    let mut transcript: BytesHashTranscript<EvalF, Keccak256hasher> = BytesHashTranscript::new();
    let mut transcript_cloned = transcript.clone();

    let orion_pcs =
        OrionPublicParams::from_random(num_vars, ORION_CODE_PARAMETER_INSTANCE, &mut rng);

    let commit_with_data = orion_pcs.commit::<F, PackF>(&random_poly).unwrap();

    let (_, opening) = orion_pcs.open(
        &random_poly,
        &commit_with_data,
        &random_point,
        &mut transcript,
    );

    // NOTE: evaluation consistency check
    let (row_num, col_num) = OrionPublicParams::row_col_from_variables(num_vars);
    let vars_for_col = log2(col_num) as usize;
    let poly_half_evaled = MultiLinearPoly::new(opening.eval_row.clone());
    let actual_eval = poly_half_evaled.evaluate_jolt(&random_point[..vars_for_col]);
    let expected_eval = random_poly_ext.evaluate_jolt(&random_point);
    assert_eq!(expected_eval, actual_eval);

    // NOTE: compute evaluation codeword
    let eval_codeword = orion_pcs.code_instance.encode(&opening.eval_row).unwrap();
    let eq_linear_combination = EqPolynomial::build_eq_x_r(&random_point[vars_for_col..]);
    let mut interleaved_codeword_ext = commit_with_data
        .interleaved_alphabet_tree
        .unpack_field_elems::<F, PackF>()
        .iter()
        .map(|&f| EvalF::from(f))
        .collect::<Vec<_>>();
    interleaved_codeword_ext.resize(row_num * orion_pcs.code_len(), EvalF::ZERO);

    let eq_combined_codeword =
        column_combination(&interleaved_codeword_ext, &eq_linear_combination);
    assert_eq!(eval_codeword, eq_combined_codeword);

    // NOTE: compute proximity codewords
    let proximity_repetitions =
        orion_pcs.proximity_repetition_num(ORION_PCS_SOUNDNESS_BITS, EvalF::FIELD_SIZE);
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
    (19..=25).for_each(|num_vars| {
        test_orion_pcs_open_generics::<GF2, GF2_128, GF2x128>(num_vars);
        test_orion_pcs_open_generics::<M31, M31Ext3, M31x16>(num_vars)
    })
}

fn test_orion_pcs_full_e2e_generics<F, EvalF, PackF>(num_vars: usize)
where
    F: Field + FieldSerde,
    EvalF: Field + FieldSerde + Mul<F, Output = EvalF> + From<F>,
    PackF: SimdField<Scalar = F>,
{
    let mut rng = test_rng();

    let random_poly = MultiLinearPoly::<F>::random(num_vars, &mut rng);
    let random_poly_ext = MultiLinearPoly::new(
        random_poly
            .coeffs
            .iter()
            .cloned()
            .map(EvalF::from)
            .collect(),
    );
    let random_point: Vec<_> = (0..num_vars)
        .map(|_| EvalF::random_bool(&mut rng))
        .collect();
    let expected_eval = random_poly_ext.evaluate_jolt(&random_point);

    let mut transcript: BytesHashTranscript<EvalF, Keccak256hasher> = BytesHashTranscript::new();
    let mut transcript_cloned = transcript.clone();

    let orion_pp =
        OrionPublicParams::from_random(num_vars, ORION_CODE_PARAMETER_INSTANCE, &mut rng);

    let commit_with_data = orion_pp.commit::<F, PackF>(&random_poly).unwrap();

    let (_, opening) = orion_pp.open(
        &random_poly,
        &commit_with_data,
        &random_point,
        &mut transcript,
    );

    assert!(
        orion_pp.verify::<F, PackF, EvalF, BytesHashTranscript<EvalF, Keccak256hasher>>(
            &commit_with_data.into(),
            &random_point,
            expected_eval,
            &opening,
            &mut transcript_cloned
        )
    );
}

#[test]
fn test_orion_pcs_full_e2e() {
    (19..=25).for_each(|num_vars| {
        test_orion_pcs_full_e2e_generics::<GF2, GF2_128, GF2x8>(num_vars);
        test_orion_pcs_full_e2e_generics::<GF2, GF2_128, GF2x64>(num_vars);
        test_orion_pcs_full_e2e_generics::<GF2, GF2_128, GF2x128>(num_vars);
        test_orion_pcs_full_e2e_generics::<M31, M31, M31x16>(num_vars);
        test_orion_pcs_full_e2e_generics::<M31, M31Ext3, M31x16>(num_vars);
    })
}
