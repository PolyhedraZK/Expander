use std::marker::PhantomData;

use arith::{Field, FieldSerde, SimdField};
use ark_std::test_rng;
use gf2::{GF2x128, GF2x512, GF2x64, GF2x8, GF2};
use polynomials::MultiLinearPoly;

use crate::orion::{
    linear_code::{OrionCode, ORION_CODE_PARAMETER_INSTANCE},
    pcs_impl::{OrionCommitment, OrionCommitmentWithData, OrionPublicParams},
    utils::{transpose_in_place, SubsetSumLUTs, TensorIOPPCS},
};

fn column_combination<F, PackF>(mat: &[F], combination: &[F]) -> Vec<F>
where
    F: Field,
    PackF: SimdField<Scalar = F>,
{
    assert_eq!(combination.len() % PackF::PACK_SIZE, 0);

    let mut luts = SubsetSumLUTs::new(PackF::PACK_SIZE, combination.len() / PackF::PACK_SIZE);
    luts.build(combination);

    mat.chunks(combination.len())
        .map(|p_col| {
            let packed: Vec<_> = p_col.chunks(PackF::PACK_SIZE).map(PackF::pack).collect();
            luts.lookup_and_sum(&packed)
        })
        .collect()
}

fn test_orion_code_generic<F, PackF>(msg_len: usize)
where
    F: Field,
    PackF: SimdField<Scalar = F>,
{
    let mut rng = test_rng();

    let orion_code = OrionCode::new(ORION_CODE_PARAMETER_INSTANCE, msg_len, &mut rng);

    let row_num = 1024 / F::FIELD_SIZE;
    let weights: Vec<_> = (0..row_num).map(|_| F::random_unsafe(&mut rng)).collect();

    // NOTE: generate message and codeword in the slice buffer
    let mut message_mat = vec![F::ZERO; row_num * orion_code.msg_len()];
    let mut codeword_mat = vec![F::ZERO; row_num * orion_code.code_len()];

    message_mat
        .chunks_mut(orion_code.msg_len())
        .zip(codeword_mat.chunks_mut(orion_code.code_len()))
        .for_each(|(msg, codeword)| {
            msg.fill_with(|| F::random_unsafe(&mut rng));
            orion_code.encode_in_place(msg, codeword).unwrap()
        });

    // NOTE: transpose message and codeword matrix
    let mut message_scratch = vec![F::ZERO; row_num * orion_code.msg_len()];
    transpose_in_place(&mut message_mat, &mut message_scratch, row_num);
    drop(message_scratch);

    let mut codeword_scratch = vec![F::ZERO; row_num * orion_code.code_len()];
    transpose_in_place(&mut codeword_mat, &mut codeword_scratch, row_num);
    drop(codeword_scratch);

    // NOTE: message and codeword matrix linear combination with weights
    let msg_linear_combined = column_combination::<F, PackF>(&message_mat, &weights);
    let codeword_linear_combined = column_combination::<F, PackF>(&codeword_mat, &weights);

    let codeword_computed = orion_code.encode(&msg_linear_combined).unwrap();

    assert_eq!(codeword_linear_combined, codeword_computed);
}

#[test]
fn test_orion_code() {
    (5..=15).for_each(|num_vars| {
        let msg_len = 1usize << num_vars;
        test_orion_code_generic::<GF2, GF2x8>(msg_len);
    });
}

fn dumb_commit<F, ComPackF>(
    orion_pp: &OrionPublicParams,
    poly: &MultiLinearPoly<F>,
) -> OrionCommitmentWithData<F, ComPackF>
where
    F: Field + FieldSerde,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = OrionPublicParams::row_col_from_variables::<F>(poly.get_num_vars());

    let mut interleaved_codewords: Vec<_> = poly
        .coeffs
        .chunks(msg_size)
        .flat_map(|msg| orion_pp.code_instance.encode(&msg).unwrap())
        .collect();

    let mut scratch = vec![F::ZERO; row_num * orion_pp.codeword_len()];
    transpose_in_place(&mut interleaved_codewords, &mut scratch, row_num);
    drop(scratch);

    if !interleaved_codewords.len().is_power_of_two() {
        let aligned_po2_len = interleaved_codewords.len().next_power_of_two();
        interleaved_codewords.resize(aligned_po2_len, F::default());
    }

    let interleaved_alphabet_tree =
        tree::Tree::compact_new_with_field_elems::<F, ComPackF>(&interleaved_codewords);

    OrionCommitmentWithData {
        interleaved_alphabet_tree,
        _phantom: PhantomData,
    }
}

fn test_orion_commit_consistency_generic<F, ComPackF>(num_vars: usize)
where
    F: Field + FieldSerde,
    ComPackF: SimdField<Scalar = F>,
{
    let mut rng = test_rng();

    let random_poly = MultiLinearPoly::<F>::random(num_vars, &mut rng);
    let orion_pcs =
        OrionPublicParams::from_random::<F>(num_vars, ORION_CODE_PARAMETER_INSTANCE, &mut rng);

    let real_commit = orion_pcs.commit::<F, ComPackF>(&random_poly).unwrap();
    let dumb_commit = dumb_commit::<F, ComPackF>(&orion_pcs, &random_poly);

    let real_commitment: OrionCommitment = real_commit.into();
    let dumb_commitment: OrionCommitment = dumb_commit.into();

    assert_eq!(real_commitment, dumb_commitment);
}

#[test]
fn test_orion_commit_consistency() {
    (19..=25).for_each(|num_vars| {
        test_orion_commit_consistency_generic::<GF2, GF2x64>(num_vars);
        test_orion_commit_consistency_generic::<GF2, GF2x128>(num_vars);
        test_orion_commit_consistency_generic::<GF2, GF2x512>(num_vars);
    });
}
