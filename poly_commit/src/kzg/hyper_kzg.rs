use std::iter;

use arith::ExtensionField;
use halo2curves::{
    ff::Field,
    group::{prime::PrimeCurveAffine, GroupEncoding},
    pairing::MultiMillerLoop,
    CurveAffine,
};
use itertools::izip;
use transcript::Transcript;

use crate::*;

#[inline(always)]
pub(crate) fn coeff_form_uni_hyperkzg_open<E: MultiMillerLoop, T: Transcript<E::Fr>>(
    srs: &CoefFormUniKZGSRS<E>,
    coeffs: &Vec<E::Fr>,
    alphas: &[E::Fr],
    fs_transcript: &mut T,
) -> (E::Fr, HyperKZGOpening<E>)
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    let mut local_coeffs = coeffs.to_vec();

    let (folded_oracle_commits, folded_oracle_coeffs): (Vec<E::G1Affine>, Vec<Vec<E::Fr>>) = alphas
        [..alphas.len() - 1]
        .iter()
        .map(|alpha| {
            local_coeffs = local_coeffs
                .chunks(2)
                .map(|c| (E::Fr::ONE - alpha) * c[0] + *alpha * c[1])
                .collect();

            let folded_oracle_commit = coeff_form_uni_kzg_commit(srs, &local_coeffs);

            fs_transcript.append_u8_slice(folded_oracle_commit.to_bytes().as_ref());

            (folded_oracle_commit, local_coeffs.clone())
        })
        .unzip();

    let beta = fs_transcript.generate_challenge_field_element();
    let beta2 = beta * beta;
    let beta_inv = beta.invert().unwrap();
    let two_inv = E::Fr::ONE.double().invert().unwrap();
    let beta_pow_series = powers_series(&beta, coeffs.len());
    let neg_beta_pow_series = powers_series(&(-beta), coeffs.len());

    let beta2_eval = {
        let beta2_pow_series = powers_series(&beta2, coeffs.len());
        univariate_evaluate(coeffs, &beta2_pow_series)
    };

    fs_transcript.append_field_element(&beta2_eval);

    let mut local_evals = HyperKZGLocalEvals::<E>::new_from_beta2_evals(beta2_eval);

    izip!(iter::once(coeffs).chain(&folded_oracle_coeffs), alphas).for_each(|(cs, alpha)| {
        let beta_eval = univariate_evaluate(cs, &beta_pow_series);
        let neg_beta_eval = univariate_evaluate(cs, &neg_beta_pow_series);

        let beta2_eval = two_inv
            * ((beta_eval + neg_beta_eval) * (E::Fr::ONE - alpha)
                + (beta_eval - neg_beta_eval) * beta_inv * alpha);

        local_evals.beta2_evals.push(beta2_eval);
        local_evals.pos_beta_evals.push(beta_eval);
        local_evals.neg_beta_evals.push(neg_beta_eval);

        fs_transcript.append_field_element(&beta_eval);
        fs_transcript.append_field_element(&neg_beta_eval);
    });

    let gamma = fs_transcript.generate_challenge_field_element();

    let (v_beta2, v_beta, v_neg_beta) = local_evals.gamma_aggregate_evals(gamma);
    let f_gamma = {
        let gamma_pow_series = powers_series(&gamma, alphas.len());
        let mut f = coeffs.clone();
        izip!(&gamma_pow_series[1..], &folded_oracle_coeffs)
            .for_each(|(gamma_i, folded_f)| polynomial_add(&mut f, *gamma_i, folded_f));
        f
    };

    let lagrange_degree2 =
        coeff_form_degree2_lagrange([beta, -beta, beta2], [v_beta, v_neg_beta, v_beta2]);
    let f_gamma_quotient = {
        let mut nom = f_gamma.clone();
        polynomial_add(&mut nom, -E::Fr::ONE, &lagrange_degree2);
        univariate_roots_quotient(nom, &[beta, beta2, -beta])
    };
    let f_gamma_quotient_com = coeff_form_uni_kzg_commit(srs, &f_gamma_quotient);
    fs_transcript.append_u8_slice(f_gamma_quotient_com.to_bytes().as_ref());

    let tau = fs_transcript.generate_challenge_field_element();
    let vanishing_at_tau = {
        let f_gamma_denom = (tau - beta) * (tau + beta) * (tau - beta2);
        let lagrange_degree2_at_tau =
            lagrange_degree2[0] + lagrange_degree2[1] * tau + lagrange_degree2[2] * tau * tau;

        let mut poly = f_gamma.clone();
        poly[0] -= lagrange_degree2_at_tau;
        polynomial_add(&mut poly, -f_gamma_denom, &f_gamma_quotient);
        univariate_roots_quotient(poly, &[tau])
    };
    let vanishing_at_tau_commitment = coeff_form_uni_kzg_commit(srs, &vanishing_at_tau);

    (
        local_evals.multilinear_final_eval(),
        HyperKZGOpening {
            folded_oracle_commitments: folded_oracle_commits,
            f_beta2: beta2_eval,
            evals_at_beta: local_evals.pos_beta_evals,
            evals_at_neg_beta: local_evals.neg_beta_evals,
            beta_commitment: f_gamma_quotient_com,
            tau_vanishing_commitment: vanishing_at_tau_commitment,
        },
    )
}

#[inline(always)]
pub(crate) fn coeff_form_uni_hyperkzg_verify<E: MultiMillerLoop, T: Transcript<E::Fr>>(
    vk: UniKZGVerifierParams<E>,
    comm: E::G1Affine,
    alphas: &[E::Fr],
    eval: E::Fr,
    opening: &HyperKZGOpening<E>,
    fs_transcript: &mut T,
) -> bool
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    opening
        .folded_oracle_commitments
        .iter()
        .for_each(|f| fs_transcript.append_u8_slice(f.to_bytes().as_ref()));

    let beta = fs_transcript.generate_challenge_field_element();
    let beta2 = beta * beta;
    let beta_inv = beta.invert().unwrap();
    let two_inv = E::Fr::ONE.double().invert().unwrap();

    fs_transcript.append_field_element(&opening.f_beta2);
    let mut beta2_evals = vec![opening.f_beta2];
    izip!(&opening.evals_at_beta, &opening.evals_at_neg_beta, alphas).for_each(
        |(beta_eval, neg_beta_eval, alpha)| {
            let beta2_eval = two_inv
                * ((*beta_eval + *neg_beta_eval) * (E::Fr::ONE - alpha)
                    + (*beta_eval - *neg_beta_eval) * beta_inv * alpha);

            beta2_evals.push(beta2_eval);

            fs_transcript.append_field_element(beta_eval);
            fs_transcript.append_field_element(neg_beta_eval);
        },
    );

    if beta2_evals[beta2_evals.len() - 1] != eval {
        return false;
    }

    let gamma = fs_transcript.generate_challenge_field_element();
    let gamma_pow_series = powers_series(&gamma, alphas.len());
    let v_beta = univariate_evaluate(&opening.evals_at_beta, &gamma_pow_series);
    let v_neg_beta = univariate_evaluate(&opening.evals_at_neg_beta, &gamma_pow_series);
    let v_beta2 = univariate_evaluate(&beta2_evals, &gamma_pow_series);
    let lagrange_degree2 =
        coeff_form_degree2_lagrange([beta, -beta, beta2], [v_beta, v_neg_beta, v_beta2]);

    let folded_g1_oracle_comms: Vec<E::G1> = opening
        .folded_oracle_commitments
        .iter()
        .map(|c| c.to_curve())
        .collect();
    let commitment_agg_g1: E::G1 =
        comm.to_curve() + univariate_evaluate(&folded_g1_oracle_comms, &gamma_pow_series[1..]);

    fs_transcript.append_u8_slice(opening.beta_commitment.to_bytes().as_ref());
    let tau = fs_transcript.generate_challenge_field_element();

    let q_weight = (tau - beta) * (tau - beta2) * (tau + beta);
    let lagrange_eval =
        lagrange_degree2[0] + lagrange_degree2[1] * tau + lagrange_degree2[2] * tau * tau;

    coeff_form_uni_kzg_verify(
        vk,
        (commitment_agg_g1 - opening.beta_commitment.to_curve() * q_weight).into(),
        tau,
        lagrange_eval,
        opening.tau_vanishing_commitment,
    )
}
