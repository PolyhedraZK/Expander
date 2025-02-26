use arith::{Field, SimdField};
use ark_std::test_rng;
use gf2::{GF2x128, GF2x64, GF2};
use mersenne31::{M31x16, M31};
use polynomials::MultiLinearPoly;

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
        .flat_map(|msg| orion_srs.code_instance.encode(msg).unwrap())
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
    (16..=19).for_each(|num_vars| {
        test_orion_commit_base_field_consistency_generic::<GF2, GF2x64>(num_vars);
        test_orion_commit_base_field_consistency_generic::<GF2, GF2x128>(num_vars);
    });

    (12..=16).for_each(|num_vars| {
        test_orion_commit_base_field_consistency_generic::<M31, M31x16>(num_vars)
    });
}
