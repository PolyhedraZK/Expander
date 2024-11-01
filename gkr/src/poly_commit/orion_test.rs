use arith::Field;
use ark_std::test_rng;
use gf2_128::GF2_128;
use mersenne31::M31Ext3;

use crate::{column_combination, transpose_in_place, OrionCode, OrionCodeParameter};

fn test_orion_code_generic<F: Field>() {
    let mut rng = test_rng();

    // NOTE: beware - this is a sketch code parameter from
    // https://eprint.iacr.org/2022/1010.pdf (Orion) p8
    // on general Spielman code.
    // This set of params might not be carefully calculated for soundness.
    // Only used here for testing purpose
    let example_orion_code_parameter = OrionCodeParameter {
        input_message_len: 1 << 10,
        output_code_len: 1 << 12,

        alpha_g0: 0.5,
        degree_g0: 6,

        lenghth_threshold_g0s: 10,

        degree_g1: 6,
    };

    let orion_code = OrionCode::new(example_orion_code_parameter, &mut rng);

    let linear_combine_size = 128;

    let random_scalrs: Vec<_> = (0..linear_combine_size)
        .map(|_| F::random_unsafe(&mut rng))
        .collect();

    // NOTE: generate message and codeword in the slice buffer

    let mut message_mat =
        vec![F::ZERO; linear_combine_size * example_orion_code_parameter.input_message_len];

    let mut codeword_mat =
        vec![F::ZERO; linear_combine_size * example_orion_code_parameter.output_code_len];

    message_mat
        .chunks_mut(example_orion_code_parameter.input_message_len)
        .zip(codeword_mat.chunks_mut(example_orion_code_parameter.output_code_len))
        .try_for_each(|(msg, codeword)| {
            msg.iter_mut().for_each(|x| *x = F::random_unsafe(&mut rng));
            orion_code.encode_in_place(msg, codeword)
        })
        .unwrap();

    // NOTE: transpose message and codeword matrix

    let mut message_scratch =
        vec![F::ZERO; linear_combine_size * example_orion_code_parameter.input_message_len];
    transpose_in_place(&mut message_mat, &mut message_scratch, linear_combine_size);
    drop(message_scratch);

    let mut codeword_scratch =
        vec![F::ZERO; linear_combine_size * example_orion_code_parameter.output_code_len];
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
    test_orion_code_generic::<M31Ext3>();
}
