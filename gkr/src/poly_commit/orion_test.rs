use arith::Field;
use ark_std::test_rng;
use gf2_128::GF2_128;

use crate::{OrionCode, OrionCodeParameter};

fn gen_msg_codeword<F: Field>(code: &OrionCode, mut rng: impl rand::RngCore) -> (Vec<F>, Vec<F>) {
    let random_msg0: Vec<_> = (0..code.msg_len())
        .map(|_| F::random_unsafe(&mut rng))
        .collect();

    let codeword0 = code.encode(&random_msg0).unwrap();

    (random_msg0, codeword0)
}

fn linear_combine<F: Field>(vec_s: &Vec<Vec<F>>, scalars: &[F]) -> Vec<F> {
    assert_eq!(vec_s.len(), scalars.len());

    let mut out = vec![F::ZERO; vec_s[0].len()];

    scalars.iter().enumerate().for_each(|(i, scalar)| {
        vec_s[i]
            .iter()
            .zip(out.iter_mut())
            .for_each(|(v_ij, o_j)| *o_j += *v_ij * scalar);
    });

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

    let orion_code = OrionCode::new(example_orion_code_parameter, &mut rng);

    let linear_combine_size = 128;

    let random_scalrs: Vec<_> = (0..linear_combine_size)
        .map(|_| F::random_unsafe(&mut rng))
        .collect();

    let (msgs, codewords): (Vec<_>, Vec<_>) = (0..linear_combine_size)
        .map(|_| gen_msg_codeword(&orion_code, &mut rng))
        .unzip();

    let msg_linear_combined = linear_combine(&msgs, &random_scalrs);
    let codeword_linear_combined = linear_combine(&codewords, &random_scalrs);

    let codeword_computed = orion_code.encode(&msg_linear_combined).unwrap();

    assert_eq!(codeword_linear_combined, codeword_computed);
}

#[test]
fn test_orion_code() {
    test_orion_code_generic::<GF2_128>()
}
