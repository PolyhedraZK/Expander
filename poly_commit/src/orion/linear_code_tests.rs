use arith::{Field, SimdField};
use ark_std::test_rng;
use gf2::{GF2x8, GF2};

use crate::{
    orion::{linear_code::OrionCode, utils::transpose_in_place},
    traits::TensorCodeIOPPCS,
    OrionSRS, SubsetSumLUTs, ORION_CODE_PARAMETER_INSTANCE,
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

    let row_bits = OrionSRS::LEAVES_IN_RANGE_OPENING * tree::LEAF_BYTES * 8;
    let row_num = row_bits / F::FIELD_SIZE;
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
