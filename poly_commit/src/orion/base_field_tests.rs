use arith::{ExtensionField, Field, SimdField};
use ark_std::test_rng;
use gf2::{GF2x128, GF2x64, GF2x8, GF2};
use gf2_128::GF2_128;
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};

use crate::{
    orion::{base_field_impl::*, utils::*},
    traits::TensorCodeIOPPCS,
    ORION_CODE_PARAMETER_INSTANCE,
};

fn dumb_commit_base_field<F, ComPackF>(
    orion_srs: &OrionSRS,
    poly: &MultiLinearPoly<F>,
) -> OrionCommitment
where
    F: Field,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = OrionSRS::evals_shape::<F>(poly.get_num_vars());

    let mut interleaved_codewords: Vec<_> = poly
        .coeffs
        .chunks(msg_size)
        .flat_map(|msg| orion_srs.code_instance.encode(&msg).unwrap())
        .collect();

    let mut scratch = vec![F::ZERO; row_num * orion_srs.codeword_len()];
    transpose_in_place(&mut interleaved_codewords, &mut scratch, row_num);
    drop(scratch);

    if !interleaved_codewords.len().is_power_of_two() {
        let aligned_po2_len = interleaved_codewords.len().next_power_of_two();
        interleaved_codewords.resize(aligned_po2_len, F::ZERO);
    }

    let interleaved_alphabet_tree =
        tree::Tree::compact_new_with_field_elems::<F, ComPackF>(interleaved_codewords);

    interleaved_alphabet_tree.root()
}

fn test_orion_commit_base_field_consistency_generic<F, ComPackF>(num_vars: usize)
where
    F: Field,
    ComPackF: SimdField<Scalar = F>,
{
    let mut rng = test_rng();

    let random_poly = MultiLinearPoly::<F>::random(num_vars, &mut rng);
    let srs = OrionSRS::from_random::<F>(num_vars, ORION_CODE_PARAMETER_INSTANCE, &mut rng);
    let mut scratch_pad = OrionScratchPad::<F, ComPackF>::default();

    let real_commitment = orion_commit_base_field(&srs, &random_poly, &mut scratch_pad).unwrap();
    let dumb_commitment = dumb_commit_base_field::<F, ComPackF>(&srs, &random_poly);

    assert_eq!(real_commitment, dumb_commitment);
}

#[test]
fn test_orion_commit_base_field_consistency() {
    (19..=25).for_each(|num_vars| {
        test_orion_commit_base_field_consistency_generic::<GF2, GF2x64>(num_vars);
        test_orion_commit_base_field_consistency_generic::<GF2, GF2x128>(num_vars);
    });
}

fn test_orion_pcs_base_full_e2e_generics<F, EvalF, ComPackF, OpenPackF>(num_vars: usize)
where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
{
    let mut rng = test_rng();

    let poly = MultiLinearPoly::<F>::random(num_vars, &mut rng);
    let poly_ext_coeffs: Vec<_> = poly.coeffs.iter().map(|t| EvalF::from(*t)).collect();
    let random_point: Vec<_> = (0..num_vars)
        .map(|_| EvalF::random_unsafe(&mut rng))
        .collect();
    let mut scratch = vec![EvalF::ZERO; 1 << num_vars];
    let expected_eval =
        MultiLinearPoly::evaluate_with_buffer(&poly_ext_coeffs, &random_point, &mut scratch);
    drop(scratch);

    let mut transcript: BytesHashTranscript<EvalF, Keccak256hasher> = BytesHashTranscript::new();
    let mut transcript_cloned = transcript.clone();

    let srs = OrionSRS::from_random::<F>(num_vars, ORION_CODE_PARAMETER_INSTANCE, &mut rng);
    let mut scratch_pad = OrionScratchPad::<F, ComPackF>::default();
    let commitment = orion_commit_base_field(&srs, &poly, &mut scratch_pad).unwrap();

    let (_, opening) = orion_open_base_field::<F, EvalF, ComPackF, OpenPackF, _>(
        &srs,
        &poly,
        &random_point,
        &mut transcript,
        &scratch_pad,
    );

    assert!(orion_verify_base_field::<F, EvalF, ComPackF, OpenPackF, _>(
        &srs,
        &commitment,
        &random_point,
        expected_eval,
        &mut transcript_cloned,
        &opening,
    ));
}

#[test]
fn test_orion_pcs_base_full_e2e() {
    (19..=25).for_each(|num_vars| {
        test_orion_pcs_base_full_e2e_generics::<GF2, GF2_128, GF2x64, GF2x8>(num_vars);
        test_orion_pcs_base_full_e2e_generics::<GF2, GF2_128, GF2x128, GF2x8>(num_vars);
    });
}
