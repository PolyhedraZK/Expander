use std::iter::once;

use arith::ExtensionField;
use halo2curves::{
    ff::Field,
    group::{prime::PrimeCurveAffine, Curve, GroupEncoding},
    pairing::MultiMillerLoop,
    CurveAffine,
};
use itertools::izip;
use transcript::Transcript;

use crate::*;
use kzg::hyper_kzg::*;

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
    let commitments: Vec<_> = izip!(srs_s, coeffs_s)
        .map(|(srs, x_coeffs)| coeff_form_uni_kzg_commit(&srs.tau_x_srs, x_coeffs))
        .collect();

    let global_commitment_g1: E::G1 = commitments.iter().map(|c| c.to_curve()).sum();
    let global_commitment: E::G1Affine = global_commitment_g1.to_affine();

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
    let lagrange_degree2_delta_y = {
        let pos_beta_y_pow_series = powers_series(&beta_y, lagrange_degree2_delta_x.len());
        let neg_beta_y_pow_series = powers_series(&(-beta_y), lagrange_degree2_delta_x.len());
        let beta_y2_pow_series = powers_series(&(beta_y * beta_y), lagrange_degree2_delta_x.len());
        let at_pos_beta_y = univariate_evaluate(&lagrange_degree2_delta_x, &pos_beta_y_pow_series);
        let at_neg_beta_y = univariate_evaluate(&lagrange_degree2_delta_x, &neg_beta_y_pow_series);
        let at_beta_y2 = univariate_evaluate(&lagrange_degree2_delta_x, &beta_y2_pow_series);
        coeff_form_degree2_lagrange(
            [beta_y, -beta_y, beta_y * beta_y],
            [at_pos_beta_y, at_neg_beta_y, at_beta_y2],
        )
    };

    // TODO(HS) vanish over the three beta_y points above, then commit Q_y
    let f_gamma_quotient_y = {
        let mut nom = lagrange_degree2_delta_x.clone();
        polynomial_add(&mut nom, -E::Fr::ONE, &lagrange_degree2_delta_y);
        univariate_roots_quotient(nom, &[beta_y, -beta_y, beta_y * beta_y])
    };
    let f_gamma_quotient_com_y =
        coeff_form_uni_kzg_commit(&srs_s[0].tau_y_srs, &f_gamma_quotient_y);

    // TODO(HS) sample from RO for delta_y
    fs_transcript.append_u8_slice(f_gamma_quotient_com_y.to_bytes().as_ref());

    let delta_y = fs_transcript.generate_challenge_field_element();

    // TODO(HS) f_gamma_s - (delta_x - beta_x) ... (delta_x - beta_x2) f_gamma_quotient_s
    //                    - (delta_y - beta_y) ... (delta_y - beta_y2) lagrange_quotient_y
    let delta_x_denom = (delta_x - beta_x) * (delta_x - beta_x * beta_x) * (delta_x + beta_x);
    let delta_y_denom = (delta_y - beta_y) * (delta_y - beta_y * beta_y) * (delta_y + beta_y);

    izip!(&mut f_gamma_s, &f_gamma_quotient_s, &f_gamma_quotient_y).for_each(
        |(f_gamma, f_gamma_quotient, f_gamma_quotient_y_i)| {
            polynomial_add(f_gamma, -delta_x_denom, &f_gamma_quotient);
            f_gamma[0] -= *f_gamma_quotient_y_i * delta_y_denom;
        },
    );

    // TODO(HS) bivariate KZG opening
    let evals_and_opens: Vec<(E::Fr, E::G1Affine)> = izip!(srs_s, &f_gamma_s)
        .map(|(srs, x_coeffs)| coeff_form_uni_kzg_open_eval(&srs.tau_x_srs, x_coeffs, delta_x))
        .collect();

    let (final_eval, final_opening) =
        coeff_form_bi_kzg_open_leader(&srs_s[0], &evals_and_opens, delta_y);

    // TODO(HS) verify openings one by one
    let vk: BiKZGVerifierParam<E> = From::from(&srs_s[0]);
    coeff_form_bi_kzg_verify(
        vk,
        global_commitment,
        delta_x,
        delta_y,
        final_eval,
        final_opening,
    );
}
