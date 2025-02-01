use arith::BN254Fr;
use ark_std::test_rng;
use field_hashers::MiMC5FiatShamirHasher;
use halo2curves::{
    bn256::{Bn256, Fr},
    ff::Field,
};
use polynomials::MultiLinearPoly;
use transcript::{FieldHashTranscript, Transcript};

use crate::{
    coeff_form_uni_hyperkzg_open, coeff_form_uni_hyperkzg_verify, coeff_form_uni_kzg_verify,
    univariate_degree_one_quotient, UniKZGVerifierParams,
};

use super::{
    coeff_form_uni_kzg_commit, coeff_form_uni_kzg_open, generate_coef_form_uni_kzg_srs_for_testing,
};

#[test]
fn test_univariate_degree_one_quotient() {
    {
        // x^3 + 1 = (x + 1)(x^2 - x + 1)
        let poly = vec![
            Fr::from(1u64),
            Fr::from(0u64),
            Fr::from(0u64),
            Fr::from(1u64),
        ];
        let point = -Fr::from(1u64);
        let (div, remainder) = univariate_degree_one_quotient(&poly, point);
        assert_eq!(
            div,
            vec![
                Fr::from(1u64),
                -Fr::from(1u64),
                Fr::from(1u64),
                Fr::from(0u64)
            ]
        );
        assert_eq!(remainder, Fr::ZERO)
    }
    {
        // x^3 - 1 = (x-1)(x^2 + x + 1)
        let poly = vec![
            -Fr::from(1u64),
            Fr::from(0u64),
            Fr::from(0u64),
            Fr::from(1u64),
        ];
        let point = Fr::from(1u64);
        let (div, remainder) = univariate_degree_one_quotient(&poly, point);
        assert_eq!(
            div,
            vec![
                Fr::from(1u64),
                Fr::from(1u64),
                Fr::from(1u64),
                Fr::from(0u64)
            ]
        );
        assert_eq!(remainder, Fr::ZERO)
    }
    {
        // x^3 + 6x^2 + 11x + 6 = (x + 1)(x + 2)(x + 3)
        let poly = vec![
            Fr::from(6u64),
            Fr::from(11u64),
            Fr::from(6u64),
            Fr::from(1u64),
        ];
        let point = Fr::from(1u64);
        let (div, remainder) = univariate_degree_one_quotient(&poly, point);
        assert_eq!(
            div,
            vec![
                Fr::from(18u64),
                Fr::from(7u64),
                Fr::from(1u64),
                Fr::from(0u64),
            ]
        );
        assert_eq!(remainder, Fr::from(24u64))
    }
}

#[test]
fn test_coefficient_form_univariate_kzg_e2e() {
    // \prod_{i \in [1, 7]} (x + i)
    let poly = vec![
        Fr::from(5040u32),
        Fr::from(13068u64),
        Fr::from(13132u64),
        Fr::from(6769u64),
        Fr::from(1960u64),
        Fr::from(322u64),
        Fr::from(28u64),
        Fr::from(1u64),
    ];
    let alpha = Fr::from(3u64);
    let eval = Fr::from(604800u64);

    let srs = generate_coef_form_uni_kzg_srs_for_testing::<Bn256>(8);
    let vk: UniKZGVerifierParams<Bn256> = From::from(&srs);
    let com = coeff_form_uni_kzg_commit(&srs, &poly);

    let opening = coeff_form_uni_kzg_open(&srs, &poly, alpha, eval);
    assert!(coeff_form_uni_kzg_verify(vk, com, alpha, eval, opening))
}

#[test]
fn test_coefficient_form_univariate_kzg_constant_e2e() {
    let poly = vec![Fr::from(100u64)];
    let alpha = Fr::from(3u64);
    let eval = Fr::from(100u64);

    let srs = generate_coef_form_uni_kzg_srs_for_testing::<Bn256>(8);
    let vk: UniKZGVerifierParams<Bn256> = From::from(&srs);
    let com = coeff_form_uni_kzg_commit(&srs, &poly);

    let opening = coeff_form_uni_kzg_open(&srs, &poly, alpha, eval);
    assert!(coeff_form_uni_kzg_verify(vk, com, alpha, eval, opening))
}

#[test]
fn test_hyperkzg_e2e() {
    let mut rng = test_rng();
    let max_vars = 15;
    let max_length = 1 << max_vars;

    let srs = generate_coef_form_uni_kzg_srs_for_testing::<Bn256>(max_length);
    (2..max_vars).for_each(|vars| {
        let multilinear = MultiLinearPoly::random(vars, &mut rng);
        let alphas: Vec<Fr> = (0..vars).map(|_| Fr::random(&mut rng)).collect();
        let eval = multilinear.evaluate_jolt(&alphas);

        let vk: UniKZGVerifierParams<Bn256> = From::from(&srs);
        let com = coeff_form_uni_kzg_commit(&srs, &multilinear.coeffs);
        let mut fs_transcript =
            FieldHashTranscript::<BN254Fr, MiMC5FiatShamirHasher<BN254Fr>>::new();

        let opening =
            coeff_form_uni_hyperkzg_open(&srs, &multilinear.coeffs, &alphas, &mut fs_transcript);

        assert!(coeff_form_uni_hyperkzg_verify(
            vk,
            com,
            &alphas,
            eval,
            &opening,
            &mut fs_transcript
        ))
    });
}
