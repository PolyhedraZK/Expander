use arith::Field;
use ark_std::test_rng;
use gf2_128::GF2_128;

use crate::{OrionCode, OrionCodeParameter};

fn gen_msg_codeword<F: Field>(
    code: &OrionCode<F>,
    mut rng: impl rand::RngCore,
) -> (Vec<F>, Vec<F>) {
    let random_msg0: Vec<_> = (0..code.msg_len())
        .map(|_| F::random_unsafe(&mut rng))
        .collect();

    let codeword0 = code.encode(&random_msg0).unwrap();

    (random_msg0, codeword0)
}

fn vec_add<F: Field>(vec0: Vec<F>, vec1: Vec<F>) -> Vec<F> {
    assert_eq!(vec0.len(), vec1.len());

    let mut out = vec![F::ZERO; vec0.len()];

    (0..vec0.len()).for_each(|i| out[i] = vec0[i] + vec1[i]);

    out
}

fn test_orion_code_generic<F: Field>() {
    let mut rng = test_rng();

    // NOTE: beware - this is a sketch code parameter from
    // https://eprint.iacr.org/2022/1010.pdf (Orion) p8
    // on general Spielman code.
    // This set of params might not be carefully calculated for soundness.
    // Only used here for testing purpose
    let example_orion_code_parameter = OrionCodeParameter {
        input_message_len: (1 << 10),
        output_code_len: (1 << 12),

        alpha_g0: 0.5,
        degree_g0: 6,

        lenghth_threshold_g0s: 10,

        degree_g1: 6,
    };

    let orion_code = OrionCode::<F>::new(example_orion_code_parameter, &mut rng);

    // TODO: linearity to random linear combination over vector spaces
    let (msg0, codeword0) = gen_msg_codeword(&orion_code, &mut rng);
    let (msg1, codeword1) = gen_msg_codeword(&orion_code, &mut rng);

    let msg_sum = vec_add(msg0, msg1);
    let codeword_sum = vec_add(codeword0, codeword1);

    let codeword_computed = orion_code.encode(&msg_sum).unwrap();

    assert_eq!(codeword_sum, codeword_computed);
}

#[test]
fn test_orion_code() {
    test_orion_code_generic::<GF2_128>()
}
