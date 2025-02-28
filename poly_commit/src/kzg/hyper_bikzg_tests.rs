use std::iter;

use arith::ExtensionField;
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

fn coeff_form_hyper_bikzg_open_simulate<E, T>(
    srs_s: &[CoefFormBiKZGLocalSRS<E>],
    coeffs_s: &[Vec<E::Fr>],
    local_alphas: &[E::Fr],
    mpi_alphas: &[E::Fr],
    fs_transcript: &mut T,
) -> HyperBiKZGOpening<E>
where
    E: MultiMillerLoop,
    T: Transcript<E::Fr>,
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
            let final_coeffs = coeffs.last().unwrap().clone();
            let final_alpha = local_alphas[local_alphas.len() - 1];

            (E::Fr::ONE - final_alpha) * final_coeffs[0] + final_alpha * final_coeffs[1]
        })
        .collect();

    let folded_y_oracle = coeff_form_uni_kzg_commit(&srs_s[0].tau_y_srs, &final_evals);

    let (folded_mpi_oracle_coms, folded_mpi_oracle_coeffs_s) =
        coeff_form_hyperkzg_local_poly_oracles(&srs_s[0].tau_y_srs, &final_evals, mpi_alphas);

    let folded_oracle_commitments = {
        let mut temp = folded_x_commits.clone();
        temp.push(folded_y_oracle);
        temp.extend_from_slice(&folded_mpi_oracle_coms);
        temp
    };

    folded_x_commits
        .iter()
        .chain(iter::once(&folded_y_oracle))
        .chain(&folded_mpi_oracle_coms)
        .for_each(|f| fs_transcript.append_u8_slice(f.to_bytes().as_ref()));

    let beta_x = fs_transcript.generate_challenge_field_element();
    let beta_y = fs_transcript.generate_challenge_field_element();

    dbg!(beta_x, beta_y);

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

    aggregated_evals.append_to_transcript(fs_transcript);

    let root_evals: HyperKZGLocalEvals<E> = coeff_form_hyperkzg_local_evals(
        &final_evals,
        &folded_mpi_oracle_coeffs_s,
        mpi_alphas,
        beta_y,
    );
    root_evals.append_to_transcript(fs_transcript);

    // NOTE(HS) check if the final eval of root evals match with mle poly evaluation
    dbg!(&root_evals.multilinear_final_eval());

    let gamma = fs_transcript.generate_challenge_field_element();

    dbg!(gamma);

    let f_gamma_global = {
        let gamma_n = gamma.pow_vartime([local_alphas.len() as u64]);
        let mut temp = coeff_form_hyperkzg_local_oracle_polys_aggregate::<E>(
            &final_evals,
            &folded_mpi_oracle_coeffs_s,
            gamma,
        );
        temp.iter_mut().for_each(|t| *t *= gamma_n);
        temp
    };
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

        izip!(&mut f_gamma_s_local, &f_gamma_global)
            .for_each(|(f_g, f_global)| f_g[0] += *f_global);

        f_gamma_s_local
    };

    let lagrange_degree2_s: Vec<[E::Fr; 3]> = izip!(local_evals_s, &f_gamma_global)
        .map(|(l, g)| {
            let mut local_degree_2 = l.interpolate_degree2_aggregated_evals(beta_x, gamma);
            local_degree_2[0] += g;
            local_degree_2
        })
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

    dbg!(delta_x);

    let lagrange_degree2_delta_x: Vec<E::Fr> = lagrange_degree2_s
        .iter()
        .map(|l| l[0] + l[1] * delta_x + l[2] * delta_x * delta_x)
        .collect();

    // NOTE(HS) interpolate at beta_y, beta_y2, -beta_y on lagrange_degree2_delta_x
    let lagrange_degree2_delta_y = {
        let pos_beta_y_pow_series = powers_series(&beta_y, lagrange_degree2_delta_x.len());
        let neg_beta_y_pow_series = powers_series(&(-beta_y), lagrange_degree2_delta_x.len());
        let beta_y2_pow_series = powers_series(&(beta_y * beta_y), lagrange_degree2_delta_x.len());
        let at_pos_beta_y = univariate_evaluate(&lagrange_degree2_delta_x, &pos_beta_y_pow_series);
        let at_neg_beta_y = univariate_evaluate(&lagrange_degree2_delta_x, &neg_beta_y_pow_series);
        let at_beta_y2 = univariate_evaluate(&lagrange_degree2_delta_x, &beta_y2_pow_series);

        dbg!(at_pos_beta_y, at_neg_beta_y, at_beta_y2);

        coeff_form_degree2_lagrange(
            [beta_y, -beta_y, beta_y * beta_y],
            [at_pos_beta_y, at_neg_beta_y, at_beta_y2],
        )
    };

    dbg!(lagrange_degree2_delta_y);

    // NOTE(HS) vanish over the three beta_y points above, then commit Q_y
    let mut f_gamma_quotient_y = {
        let mut nom = lagrange_degree2_delta_x.clone();
        polynomial_add(&mut nom, -E::Fr::ONE, &lagrange_degree2_delta_y);
        univariate_roots_quotient(nom, &[beta_y, -beta_y, beta_y * beta_y])
    };
    f_gamma_quotient_y.resize(lagrange_degree2_delta_x.len(), E::Fr::ZERO);

    let f_gamma_quotient_com_y =
        coeff_form_uni_kzg_commit(&srs_s[0].tau_y_srs, &f_gamma_quotient_y);

    dbg!(f_gamma_quotient_y.len());

    // NOTE(HS) sample from RO for delta_y
    fs_transcript.append_u8_slice(f_gamma_quotient_com_y.to_bytes().as_ref());

    let delta_y = fs_transcript.generate_challenge_field_element();

    dbg!(delta_y);

    // NOTE(HS) f_gamma_s - (delta_x - beta_x) ... (delta_x - beta_x2) f_gamma_quotient_s
    //                    - (delta_y - beta_y) ... (delta_y - beta_y2) lagrange_quotient_y
    let delta_x_denom = (delta_x - beta_x) * (delta_x - beta_x * beta_x) * (delta_x + beta_x);
    let delta_y_denom = (delta_y - beta_y) * (delta_y - beta_y * beta_y) * (delta_y + beta_y);

    izip!(&mut f_gamma_s, &f_gamma_quotient_s, &f_gamma_quotient_y).for_each(
        |(f_gamma, f_gamma_quotient, f_gamma_quotient_y_i)| {
            polynomial_add(f_gamma, -delta_x_denom, &f_gamma_quotient);
            f_gamma[0] -= *f_gamma_quotient_y_i * delta_y_denom;
        },
    );

    // NOTE(HS) bivariate KZG opening
    let evals_and_opens: Vec<(E::Fr, E::G1Affine)> = izip!(srs_s, &f_gamma_s)
        .map(|(srs, x_coeffs)| coeff_form_uni_kzg_open_eval(&srs.tau_x_srs, x_coeffs, delta_x))
        .collect();

    let (_, final_opening) = coeff_form_bi_kzg_open_leader(&srs_s[0], &evals_and_opens, delta_y);

    HyperBiKZGOpening {
        folded_oracle_commitments,
        aggregated_evals,
        leader_evals: root_evals.into(),
        beta_x_commitment: f_gamma_quotient_com_x,
        beta_y_commitment: f_gamma_quotient_com_y,
        quotient_delta_x_commitment: final_opening.quotient_x,
        quotient_delta_y_commitment: final_opening.quotient_y,
    }
}

fn coeff_form_hyper_bikzg_verify_simulate<E, T>(
    vk: &BiKZGVerifierParam<E>,
    local_alphas: &[E::Fr],
    mpi_alphas: &[E::Fr],
    eval: E::Fr,
    commitment: E::G1Affine,
    opening: &HyperBiKZGOpening<E>,
    fs_transcript: &mut T,
) -> bool
where
    E: MultiMillerLoop,
    T: Transcript<E::Fr>,
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    opening
        .folded_oracle_commitments
        .iter()
        .for_each(|f| fs_transcript.append_u8_slice(f.to_bytes().as_ref()));

    let beta_x = fs_transcript.generate_challenge_field_element();
    let beta_y = fs_transcript.generate_challenge_field_element();

    dbg!(beta_x, beta_y);

    // NOTE(HS) evaluation checks

    let beta_y2_local = HyperKZGLocalEvals::new_from_exported_evals(
        &opening.aggregated_evals.beta_y2_evals,
        local_alphas,
        beta_x,
    );

    let pos_beta_y_local = HyperKZGLocalEvals::new_from_exported_evals(
        &opening.aggregated_evals.pos_beta_y_evals,
        local_alphas,
        beta_x,
    );

    let neg_beta_y_local = HyperKZGLocalEvals::new_from_exported_evals(
        &opening.aggregated_evals.neg_beta_y_evals,
        local_alphas,
        beta_x,
    );

    let beta_y2_final_eval = beta_y2_local.multilinear_final_eval();
    let pos_beta_y_final_eval = pos_beta_y_local.multilinear_final_eval();
    let neg_beta_y_final_eval = neg_beta_y_local.multilinear_final_eval();

    dbg!(
        &beta_y2_final_eval,
        &pos_beta_y_final_eval,
        &neg_beta_y_final_eval
    );

    dbg!(
        &opening.leader_evals.beta_x2_eval,
        &opening.leader_evals.pos_beta_x_evals[0],
        &opening.leader_evals.neg_beta_x_evals[0]
    );

    if beta_y2_final_eval != opening.leader_evals.beta_x2_eval {
        return false;
    }
    if pos_beta_y_final_eval != opening.leader_evals.pos_beta_x_evals[0] {
        return false;
    }
    if neg_beta_y_final_eval != opening.leader_evals.neg_beta_x_evals[0] {
        return false;
    }

    let local_final_eval =
        HyperKZGLocalEvals::new_from_exported_evals(&opening.leader_evals, mpi_alphas, beta_y);
    if eval != local_final_eval.multilinear_final_eval() {
        return false;
    }

    opening.aggregated_evals.append_to_transcript(fs_transcript);
    opening.leader_evals.append_to_transcript(fs_transcript);

    let gamma = fs_transcript.generate_challenge_field_element();

    dbg!(gamma);

    let aggregated_oracle_commitment: E::G1Affine = {
        let gamma_power_series = powers_series(&gamma, local_alphas.len() + mpi_alphas.len() + 1);

        let com_g1: E::G1 = izip!(
            iter::once(&commitment).chain(&opening.folded_oracle_commitments),
            &gamma_power_series
        )
        .map(|(com, g)| com.to_curve() * g)
        .sum();

        com_g1.into()
    };
    dbg!(aggregated_oracle_commitment);

    // NOTE(HS) aggregate lagrange degree 2 polys
    let (y_beta2, y_beta, y_neg_beta) = {
        let gamma_n = gamma.pow_vartime([local_alphas.len() as u64]);
        let (v_beta2, v_beta, v_neg_beta) = local_final_eval.gamma_aggregate_evals(gamma);

        (v_beta2 * gamma_n, v_beta * gamma_n, v_neg_beta * gamma_n)
    };

    let mut aggregated_beta_y2_locals =
        beta_y2_local.interpolate_degree2_aggregated_evals(beta_x, gamma);
    aggregated_beta_y2_locals[0] += y_beta2;

    let mut aggregated_pos_beta_y_locals =
        pos_beta_y_local.interpolate_degree2_aggregated_evals(beta_x, gamma);
    aggregated_pos_beta_y_locals[0] += y_beta;

    let mut aggregated_neg_beta_y_locals =
        neg_beta_y_local.interpolate_degree2_aggregated_evals(beta_x, gamma);
    aggregated_neg_beta_y_locals[0] += y_neg_beta;

    fs_transcript.append_u8_slice(opening.beta_x_commitment.to_bytes().as_ref());

    let delta_x = fs_transcript.generate_challenge_field_element();

    dbg!(delta_x);

    let at_beta_y2 = aggregated_beta_y2_locals[0]
        + aggregated_beta_y2_locals[1] * delta_x
        + aggregated_beta_y2_locals[2] * delta_x * delta_x;

    let at_beta_y = aggregated_pos_beta_y_locals[0]
        + aggregated_pos_beta_y_locals[1] * delta_x
        + aggregated_pos_beta_y_locals[2] * delta_x * delta_x;

    let at_neg_beta_y = aggregated_neg_beta_y_locals[0]
        + aggregated_neg_beta_y_locals[1] * delta_x
        + aggregated_neg_beta_y_locals[2] * delta_x * delta_x;

    dbg!(at_beta_y2, at_beta_y, at_neg_beta_y);

    let lagrange_degree2_delta_y = coeff_form_degree2_lagrange(
        [beta_y, -beta_y, beta_y * beta_y],
        [at_beta_y, at_neg_beta_y, at_beta_y2],
    );

    dbg!(lagrange_degree2_delta_y);

    fs_transcript.append_u8_slice(opening.beta_y_commitment.to_bytes().as_ref());

    let delta_y = fs_transcript.generate_challenge_field_element();

    dbg!(delta_y);

    let degree_2_final_eval = lagrange_degree2_delta_y[0]
        + lagrange_degree2_delta_y[1] * delta_y
        + lagrange_degree2_delta_y[2] * delta_y * delta_y;

    dbg!(degree_2_final_eval);

    // NOTE(HS) f_gamma_s - (delta_x - beta_x) ... (delta_x - beta_x2) f_gamma_quotient_s
    //                    - (delta_y - beta_y) ... (delta_y - beta_y2) lagrange_quotient_y
    let delta_x_denom = (delta_x - beta_x) * (delta_x - beta_x * beta_x) * (delta_x + beta_x);
    let delta_y_denom = (delta_y - beta_y) * (delta_y - beta_y * beta_y) * (delta_y + beta_y);

    let com_r = aggregated_oracle_commitment.to_curve()
        - (opening.beta_x_commitment * delta_x_denom)
        - (opening.beta_y_commitment * delta_y_denom);

    dbg!(com_r);

    let final_opening = BiKZGProof {
        quotient_x: opening.quotient_delta_x_commitment,
        quotient_y: opening.quotient_delta_y_commitment,
    };

    let what = coeff_form_bi_kzg_verify(
        vk.clone(),
        com_r.to_affine(),
        delta_x,
        delta_y,
        degree_2_final_eval,
        final_opening,
    );
    dbg!(what);

    what
}

#[test]
fn test_hyper_bikzg_single_process_simulated_e2e() {
    let (x_degree, x_vars) = {
        let x_vars = 10;
        ((1 << x_vars) - 1, x_vars)
    };

    let (y_degree, y_vars) = {
        let y_vars = 4;
        ((1 << y_vars) - 1, y_vars)
    };

    let mut rng = test_rng();

    let local_alphas: Vec<_> = (0..x_vars).map(|_| Fr::random(&mut rng)).collect();
    let mpi_alphas: Vec<_> = (0..y_vars).map(|_| Fr::random(&mut rng)).collect();

    let party_srs: Vec<CoefFormBiKZGLocalSRS<Bn256>> = (0..=y_degree)
        .map(|rank| {
            let mut srs_rng = test_rng();
            generate_coef_form_bi_kzg_local_srs_for_testing(
                x_degree + 1,
                y_degree + 1,
                rank,
                &mut srs_rng,
            )
        })
        .collect();

    let xy_coeffs: Vec<Vec<Fr>> = (0..=y_degree)
        .map(|_| (0..=x_degree).map(|_| Fr::random(&mut rng)).collect())
        .collect();

    let all_alphas = {
        let mut alphas = local_alphas.clone();
        alphas.extend_from_slice(&mpi_alphas);

        alphas
    };

    let eval = {
        let global_poly_coeffs: Vec<_> = xy_coeffs.clone().into_iter().flatten().collect();
        let global_poly = MultiLinearPoly::new(global_poly_coeffs);
        global_poly.evaluate_jolt(&all_alphas)
    };

    dbg!(eval);

    let global_commitment: G1Affine = {
        let commitments: Vec<_> = izip!(&party_srs, &xy_coeffs)
            .map(|(srs, x_coeffs)| coeff_form_uni_kzg_commit(&srs.tau_x_srs, x_coeffs))
            .collect();

        let global_commitment_g1: G1 = commitments.iter().map(|c| c.to_curve()).sum();
        global_commitment_g1.to_affine()
    };

    let mut fs_transcript = FieldHashTranscript::<Fr, MiMC5FiatShamirHasher<Fr>>::new();
    let mut verifier_transcript = fs_transcript.clone();

    let opening = coeff_form_hyper_bikzg_open_simulate(
        &party_srs,
        &xy_coeffs,
        &local_alphas,
        &mpi_alphas,
        &mut fs_transcript,
    );

    let vk: BiKZGVerifierParam<Bn256> = From::from(&party_srs[0]);
    let what = coeff_form_hyper_bikzg_verify_simulate(
        &vk,
        &local_alphas,
        &mpi_alphas,
        eval,
        global_commitment,
        &opening,
        &mut verifier_transcript,
    );

    assert!(what);
}
