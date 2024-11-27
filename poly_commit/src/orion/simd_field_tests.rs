use arith::{ExtensionField, Field, SimdField};
use ark_std::test_rng;
use gf2::{GF2x128, GF2x64, GF2x8, GF2};
use gf2_128::GF2_128;
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};

use crate::{
    orion::{simd_field_impl::*, utils::*},
    traits::TensorCodeIOPPCS,
    ORION_CODE_PARAMETER_INSTANCE,
};

fn dumb_commit_simd_field<F, SimdF, ComPackF>(
    orion_srs: &OrionSRS,
    poly: &MultiLinearPoly<SimdF>,
) -> OrionCommitment
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = {
        let num_vars = poly.get_num_vars() + SimdF::PACK_SIZE.ilog2() as usize;
        let (row_field_elems, msg_size) = OrionSRS::evals_shape::<F>(num_vars);
        let row_num = row_field_elems / SimdF::PACK_SIZE;
        (row_num, msg_size)
    };

    let mut interleaved_codewords: Vec<_> = poly
        .coeffs
        .chunks(msg_size)
        .flat_map(|msg| orion_srs.code_instance.encode(&msg).unwrap())
        .collect();

    let mut scratch = vec![SimdF::ZERO; row_num * orion_srs.codeword_len()];
    transpose_in_place(&mut interleaved_codewords, &mut scratch, row_num);
    drop(scratch);

    let mut packed_interleaved_codeword: Vec<_> = interleaved_codewords
        .chunks(ComPackF::PACK_SIZE / SimdF::PACK_SIZE)
        .map(ComPackF::pack_from_simd)
        .collect();
    drop(interleaved_codewords);

    if !packed_interleaved_codeword.len().is_power_of_two() {
        let aligned_po2_len = packed_interleaved_codeword.len().next_power_of_two();
        packed_interleaved_codeword.resize(aligned_po2_len, ComPackF::ZERO);
    }

    let interleaved_alphabet_tree =
        tree::Tree::compact_new_with_packed_field_elems::<F, ComPackF>(packed_interleaved_codeword);

    interleaved_alphabet_tree.root()
}

fn test_orion_commit_simd_field_consistency_generic<F, SimdF, ComPackF>(num_vars: usize)
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let mut rng = test_rng();

    let random_poly = MultiLinearPoly::<SimdF>::random(num_vars, &mut rng);
    let real_num_vars = num_vars + SimdF::PACK_SIZE.ilog2() as usize;
    let srs =
        OrionSRS::from_random::<SimdF>(real_num_vars, ORION_CODE_PARAMETER_INSTANCE, &mut rng);
    let mut scratch_pad = OrionScratchPad::<F, ComPackF>::default();

    let real_commitment = orion_commit_simd_field(&srs, &random_poly, &mut scratch_pad).unwrap();
    let dumb_commitment = dumb_commit_simd_field::<F, SimdF, ComPackF>(&srs, &random_poly);

    assert_eq!(real_commitment, dumb_commitment);
}

#[test]
fn test_orion_commit_simd_field_consistency() {
    (16..=22).for_each(|num_vars| {
        test_orion_commit_simd_field_consistency_generic::<GF2, GF2x8, GF2x8>(num_vars);
        test_orion_commit_simd_field_consistency_generic::<GF2, GF2x8, GF2x64>(num_vars);
        test_orion_commit_simd_field_consistency_generic::<GF2, GF2x8, GF2x128>(num_vars);
    });
}

fn test_orion_pcs_simd_full_e2e_generics<F, SimdF, EvalF, ComPackF, OpenPackF>(num_vars: usize)
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
{
    let mut rng = test_rng();

    let random_poly = MultiLinearPoly::<SimdF>::random(num_vars, &mut rng);
    let random_poly_unpacked = MultiLinearPoly::<EvalF>::new(
        random_poly
            .coeffs
            .iter()
            .flat_map(|p| -> Vec<_> { p.unpack().iter().map(|t| EvalF::from(*t)).collect() })
            .collect(),
    );
    let real_num_vars = num_vars + SimdF::PACK_SIZE.ilog2() as usize;
    let random_point: Vec<_> = (0..real_num_vars)
        .map(|_| EvalF::random_unsafe(&mut rng))
        .collect();

    let mut scratch = vec![EvalF::ZERO; random_poly_unpacked.coeffs.len()];
    let expected_eval = MultiLinearPoly::evaluate_with_buffer(
        &random_poly_unpacked.coeffs,
        &random_point,
        &mut scratch,
    );
    drop(scratch);

    let srs =
        OrionSRS::from_random::<SimdF>(real_num_vars, ORION_CODE_PARAMETER_INSTANCE, &mut rng);
    let mut scratch_pad = OrionScratchPad::<F, ComPackF>::default();
    let mut transcript: BytesHashTranscript<EvalF, Keccak256hasher> = BytesHashTranscript::new();
    let mut transcript_cloned = transcript.clone();

    let commitment = orion_commit_simd_field(&srs, &random_poly, &mut scratch_pad).unwrap();

    let opening = orion_open_simd_field::<F, SimdF, EvalF, ComPackF, OpenPackF, _>(
        &srs,
        &random_poly,
        &random_point,
        &mut transcript,
        &scratch_pad,
    );

    assert!(
        orion_verify_simd_field::<F, SimdF, _, ComPackF, OpenPackF, _>(
            &srs,
            &commitment,
            &random_point,
            expected_eval,
            &mut transcript_cloned,
            &opening
        )
    );
}

#[test]
fn test_orion_pcs_simd_full_e2e() {
    (16..=22).for_each(|num_vars| {
        test_orion_pcs_simd_full_e2e_generics::<GF2, GF2x8, GF2_128, GF2x64, GF2x8>(num_vars);
        test_orion_pcs_simd_full_e2e_generics::<GF2, GF2x8, GF2_128, GF2x128, GF2x8>(num_vars);
    })
}
