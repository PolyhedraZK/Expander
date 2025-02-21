use std::iter::once;

use arith::{BN254Fr, ExtensionField};
use ark_std::test_rng;
use field_hashers::MiMC5FiatShamirHasher;
use halo2curves::{
    bn256::{Bn256, Fr, G1Affine, G1},
    ff::Field,
    group::{prime::PrimeCurveAffine, Curve, GroupEncoding},
    pairing::MultiMillerLoop,
    CurveAffine,
};
use itertools::izip;
use polynomials::MultiLinearPoly;
use transcript::{FieldHashTranscript, Transcript};

use crate::*;
use kzg::hyper_kzg::*;

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

    let mut rng = test_rng();
    let srs = generate_coef_form_uni_kzg_srs_for_testing::<Bn256>(8, &mut rng);
    let vk: UniKZGVerifierParams<Bn256> = From::from(&srs);
    let com = coeff_form_uni_kzg_commit(&srs, &poly);

    let (actual_eval, opening) = coeff_form_uni_kzg_open_eval(&srs, &poly, alpha);
    assert_eq!(actual_eval, eval);
    assert!(coeff_form_uni_kzg_verify(vk, com, alpha, eval, opening))
}

#[test]
fn test_coefficient_form_univariate_kzg_constant_e2e() {
    let poly = vec![Fr::from(100u64)];
    let alpha = Fr::from(3u64);
    let eval = Fr::from(100u64);

    let mut rng = test_rng();
    let srs = generate_coef_form_uni_kzg_srs_for_testing::<Bn256>(8, &mut rng);
    let vk: UniKZGVerifierParams<Bn256> = From::from(&srs);
    let com = coeff_form_uni_kzg_commit(&srs, &poly);

    let (actual_eval, opening) = coeff_form_uni_kzg_open_eval(&srs, &poly, alpha);
    assert_eq!(actual_eval, eval);
    assert!(coeff_form_uni_kzg_verify(vk, com, alpha, eval, opening))
}

#[test]
fn test_coefficient_form_bivariate_kzg_e2e() {
    let x_degree = 15;
    let y_degree = 7;

    let party_srs: Vec<CoefFormBiKZGLocalSRS<Bn256>> = (0..=y_degree)
        .map(|rank| {
            let mut rng = test_rng();
            generate_coef_form_bi_kzg_local_srs_for_testing(
                x_degree + 1,
                y_degree + 1,
                rank,
                &mut rng,
            )
        })
        .collect();

    let mut rng = test_rng();
    let xy_coeffs: Vec<Vec<Fr>> = (0..=y_degree)
        .map(|_| (0..=x_degree).map(|_| Fr::random(&mut rng)).collect())
        .collect();

    let commitments: Vec<_> = izip!(&party_srs, &xy_coeffs)
        .map(|(srs, x_coeffs)| coeff_form_uni_kzg_commit(&srs.tau_x_srs, x_coeffs))
        .collect();

    let global_commitment_g1: G1 = commitments.iter().map(|c| c.to_curve()).sum::<G1>();
    let global_commitment: G1Affine = global_commitment_g1.to_affine();

    let alpha = Fr::random(&mut rng);
    let evals_and_opens: Vec<(Fr, G1Affine)> = izip!(&party_srs, &xy_coeffs)
        .map(|(srs, x_coeffs)| coeff_form_uni_kzg_open_eval(&srs.tau_x_srs, x_coeffs, alpha))
        .collect();

    let beta = Fr::random(&mut rng);
    let (final_eval, final_opening) =
        coeff_form_bi_kzg_open_leader(&party_srs[0], &evals_and_opens, beta);

    let vk: BiKZGVerifierParam<Bn256> = From::from(&party_srs[0]);
    assert!(coeff_form_bi_kzg_verify(
        vk,
        global_commitment,
        alpha,
        beta,
        final_eval,
        final_opening,
    ));
}

#[test]
fn test_hyperkzg_functionality_e2e() {
    let mut rng = test_rng();
    let max_vars = 15;
    let max_length = 1 << max_vars;

    let srs = generate_coef_form_uni_kzg_srs_for_testing::<Bn256>(max_length, &mut rng);
    (2..max_vars).for_each(|vars| {
        let multilinear = MultiLinearPoly::random(vars, &mut rng);
        let alphas: Vec<Fr> = (0..vars).map(|_| Fr::random(&mut rng)).collect();

        let vk: UniKZGVerifierParams<Bn256> = From::from(&srs);
        let com = coeff_form_uni_kzg_commit(&srs, &multilinear.coeffs);
        let mut fs_transcript =
            FieldHashTranscript::<BN254Fr, MiMC5FiatShamirHasher<BN254Fr>>::new();

        let (eval, opening) =
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

#[allow(unused)]
fn coeff_form_hyper_bikzg_open_simulate<E: MultiMillerLoop, T: Transcript<E::Fr>>(
    srs_s: &[CoefFormBiKZGLocalSRS<E>],
    coeffs_s: &[Vec<E::Fr>],
    local_alphas: &[E::Fr],
    mpi_alphas: &[E::Fr],
    fs_transcript: &mut T,
) where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    let (folded_oracle_commits_s, folded_oracle_coeffs_s): (
        Vec<Vec<E::G1Affine>>,
        Vec<Vec<Vec<E::Fr>>>,
    ) = izip!(srs_s, coeffs_s)
        .map(|(srs, coeffs)| {
            coeff_form_hyperkzg_local_poly_oracles(&srs.tau_x_srs, coeffs, local_alphas)
        })
        .unzip();

    let folded_x_commits: Vec<E::G1Affine> = (0..local_alphas.len() - 1)
        .map(|i| {
            let ith_fold_commits: E::G1 = folded_oracle_commits_s
                .iter()
                .map(|f| f[i].to_curve())
                .sum();

            ith_fold_commits.to_affine()
        })
        .collect();

    let final_evals: Vec<E::Fr> = folded_oracle_coeffs_s
        .iter()
        .map(|coeffs| {
            let final_coeffs = coeffs[coeffs.len() - 1].clone();
            let final_alpha = local_alphas[local_alphas.len() - 1];
            final_coeffs[0] * (E::Fr::ONE - final_alpha) * final_coeffs[0]
                + final_alpha * final_coeffs[1]
        })
        .collect();

    let folded_y_oracle = coeff_form_uni_kzg_commit(&srs_s[0].tau_y_srs, &final_evals);

    let (folded_mpi_oracle_commits_s, folded_mpi_oracle_coeffs_s) =
        coeff_form_hyperkzg_local_poly_oracles(&srs_s[0].tau_y_srs, &final_evals, mpi_alphas);

    folded_x_commits
        .iter()
        .chain(once(&folded_y_oracle))
        .chain(&folded_mpi_oracle_commits_s)
        .for_each(|f| {
            fs_transcript.append_u8_slice(f.to_bytes().as_ref());
        });

    let beta_x = fs_transcript.generate_challenge_field_element();
    let beta_y = fs_transcript.generate_challenge_field_element();

    let local_evals_s: Vec<HyperKZGLocalEvals<E>> = izip!(coeffs_s, &folded_oracle_coeffs_s)
        .map(|(coeffs, folded_oracle_coeffs)| {
            coeff_form_hyperkzg_local_evals(coeffs, folded_oracle_coeffs, local_alphas, beta_x)
        })
        .collect();

    let exported_local_evals_s: Vec<_> = local_evals_s
        .iter()
        .map(|w| Into::<HyperKZGExportedLocalEvals<E>>::into(w.clone()))
        .collect();

    let aggregated_evals =
        HyperKZGAggregatedEvals::new_from_exported_evals(&exported_local_evals_s, beta_y);
    aggregated_evals
        .beta_y2_evals
        .append_to_transcript(fs_transcript);
    aggregated_evals
        .pos_beta_y_evals
        .append_to_transcript(fs_transcript);
    aggregated_evals
        .neg_beta_y_evals
        .append_to_transcript(fs_transcript);

    let root_evals: HyperKZGLocalEvals<E> = coeff_form_hyperkzg_local_evals(
        &final_evals,
        &folded_mpi_oracle_coeffs_s,
        mpi_alphas,
        beta_y,
    );
    root_evals.append_to_transcript(fs_transcript);

    let gamma = fs_transcript.generate_challenge_field_element();

    let mut f_gamma_s: Vec<Vec<E::Fr>> = {
        let mut f_gamma_s_local: Vec<Vec<E::Fr>> = izip!(coeffs_s, folded_oracle_coeffs_s)
            .map(|(coeffs, folded_oracle_coeffs)| {
                coeff_form_hyperkzg_local_oracle_polys_aggregate::<E>(
                    coeffs,
                    &folded_oracle_coeffs,
                    gamma,
                )
            })
            .collect();

        let f_gamma_global = coeff_form_hyperkzg_local_oracle_polys_aggregate::<E>(
            &final_evals,
            &folded_mpi_oracle_coeffs_s,
            gamma,
        );

        let gamma_n = gamma.pow_vartime([local_alphas.len() as u64]);

        izip!(&mut f_gamma_s_local, &f_gamma_global)
            .for_each(|(f_g, f_global)| f_g[0] += *f_global * gamma_n);

        f_gamma_s_local
    };

    let lagrange_degree2_s: Vec<[E::Fr; 3]> = local_evals_s
        .iter()
        .map(|l| l.interpolate_degree2_aggregated_evals(beta_x, gamma))
        .collect();
    let f_gamma_quotient_s: Vec<Vec<E::Fr>> = izip!(&f_gamma_s, &lagrange_degree2_s)
        .map(|(f_gamma, lagrange_degree2)| {
            let mut nom = f_gamma.clone();
            polynomial_add(&mut nom, -E::Fr::ONE, lagrange_degree2);
            univariate_roots_quotient(nom, &[beta_x, -beta_x, beta_x * beta_x])
        })
        .collect();
    let f_gamma_quotient_com_s: Vec<E::G1> = izip!(srs_s, &f_gamma_quotient_s)
        .map(|(srs, f_gamma_quotient)| {
            coeff_form_uni_kzg_commit(&srs.tau_x_srs, f_gamma_quotient).to_curve()
        })
        .collect();
    let f_gamma_quotient_com_x: E::G1Affine = f_gamma_quotient_com_s.iter().sum::<E::G1>().into();

    fs_transcript.append_u8_slice(f_gamma_quotient_com_x.to_bytes().as_ref());

    let delta_x = fs_transcript.generate_challenge_field_element();

    let lagrange_degree2_delta_x: Vec<E::Fr> = lagrange_degree2_s
        .iter()
        .map(|l| l[0] + l[1] * delta_x + l[2] * delta_x * delta_x)
        .collect();

    // TODO(HS) interpolate at beta_y, beta_y2, -beta_y on lagrange_degree2_delta_x
    // TODO(HS) vanish over the three beta_y points above, then commit Q_y
    // TODO(HS) sample from RO for delta_y

    // TODO(HS) f_gamma_s - (delta_x - beta_x) ... (delta_x - beta_x2) f_gamma_quotient_s
    //                    - (delta_y - beta_u) ... (delta_y - beta_y2) lagrange_quotient_y
    // TODO(HS) bivariate KZG opening

    todo!()
}
