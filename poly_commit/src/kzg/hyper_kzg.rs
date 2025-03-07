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
pub(crate) fn coeff_form_hyperkzg_local_poly_oracles<E>(
    srs: &CoefFormUniKZGSRS<E>,
    coeffs: &[E::Fr],
    local_alphas: &[E::Fr],
) -> (Vec<E::G1Affine>, Vec<Vec<E::Fr>>)
where
    E: MultiMillerLoop,
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    let mut local_coeffs = coeffs.to_vec();

    local_alphas[..local_alphas.len() - 1]
        .iter()
        .map(|alpha| {
            local_coeffs = local_coeffs
                .chunks(2)
                .map(|c| (E::Fr::ONE - alpha) * c[0] + *alpha * c[1])
                .collect();

            let folded_oracle_commit = coeff_form_uni_kzg_commit(srs, &local_coeffs);

            (folded_oracle_commit, local_coeffs.clone())
        })
        .unzip()
}

#[inline(always)]
pub(crate) fn coeff_form_hyperkzg_local_evals<E>(
    coeffs: &Vec<E::Fr>,
    folded_oracle_coeffs: &[Vec<E::Fr>],
    local_alphas: &[E::Fr],
    beta: E::Fr,
) -> HyperKZGLocalEvals<E>
where
    E: MultiMillerLoop,
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    let beta2 = beta * beta;
    let beta_inv = beta.invert().unwrap();
    let two_inv = E::Fr::ONE.double().invert().unwrap();
    let beta_pow_series = powers_series(&beta, coeffs.len());
    let neg_beta_pow_series = powers_series(&(-beta), coeffs.len());

    let beta2_eval = {
        let beta2_pow_series = powers_series(&beta2, coeffs.len());
        univariate_evaluate(coeffs, &beta2_pow_series)
    };

    let mut local_evals = HyperKZGLocalEvals::<E>::new_from_beta2_evals(beta2_eval);

    izip!(iter::once(coeffs).chain(folded_oracle_coeffs), local_alphas).for_each(|(cs, alpha)| {
        let beta_eval = univariate_evaluate(cs, &beta_pow_series);
        let neg_beta_eval = univariate_evaluate(cs, &neg_beta_pow_series);

        let beta2_eval = two_inv
            * ((beta_eval + neg_beta_eval) * (E::Fr::ONE - alpha)
                + (beta_eval - neg_beta_eval) * beta_inv * alpha);

        local_evals.beta2_evals.push(beta2_eval);
        local_evals.pos_beta_evals.push(beta_eval);
        local_evals.neg_beta_evals.push(neg_beta_eval);
    });

    local_evals
}

#[inline(always)]
pub(crate) fn coeff_form_hyperkzg_local_oracle_polys_aggregate<E>(
    coeffs: &[E::Fr],
    folded_oracle_coeffs: &[Vec<E::Fr>],
    gamma: E::Fr,
) -> Vec<E::Fr>
where
    E: MultiMillerLoop,
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    let gamma_pow_series = powers_series(&gamma, folded_oracle_coeffs.len() + 1);
    let mut f = coeffs.to_vec();
    izip!(&gamma_pow_series[1..], folded_oracle_coeffs)
        .for_each(|(gamma_i, folded_f)| polynomial_add(&mut f, *gamma_i, folded_f));
    f
}

#[inline(always)]
pub fn coeff_form_uni_hyperkzg_open<E, T>(
    srs: &CoefFormUniKZGSRS<E>,
    coeffs: &Vec<E::Fr>,
    alphas: &[E::Fr],
    fs_transcript: &mut T,
) -> (E::Fr, HyperKZGOpening<E>)
where
    E: MultiMillerLoop,
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
    T: Transcript<E::Fr>,
{
    let (folded_oracle_commitments, folded_oracle_coeffs) =
        coeff_form_hyperkzg_local_poly_oracles(srs, coeffs, alphas);

    folded_oracle_commitments.iter().for_each(|f| {
        fs_transcript.append_u8_slice(f.to_bytes().as_ref());
    });

    let beta = fs_transcript.generate_challenge_field_element();
    let beta2 = beta * beta;

    let local_evals =
        coeff_form_hyperkzg_local_evals::<E>(coeffs, &folded_oracle_coeffs, alphas, beta);
    local_evals.append_to_transcript(fs_transcript);

    let gamma = fs_transcript.generate_challenge_field_element();

    let mut f_gamma =
        coeff_form_hyperkzg_local_oracle_polys_aggregate::<E>(coeffs, &folded_oracle_coeffs, gamma);
    let lagrange_degree2 = local_evals.interpolate_degree2_aggregated_evals(beta, gamma);
    let f_gamma_quotient = {
        let mut nom = f_gamma.clone();
        polynomial_add(&mut nom, -E::Fr::ONE, &lagrange_degree2);
        univariate_roots_quotient(nom, &[beta, beta2, -beta])
    };
    let beta_x_commitment = coeff_form_uni_kzg_commit(srs, &f_gamma_quotient);

    fs_transcript.append_u8_slice(beta_x_commitment.to_bytes().as_ref());

    let tau = fs_transcript.generate_challenge_field_element();
    let vanishing_at_tau = {
        let f_gamma_denom = (tau - beta) * (tau + beta) * (tau - beta2);
        let lagrange_degree2_at_tau =
            lagrange_degree2[0] + lagrange_degree2[1] * tau + lagrange_degree2[2] * tau * tau;

        polynomial_add(&mut f_gamma, -f_gamma_denom, &f_gamma_quotient);
        let (coeffs, remainder) = univariate_degree_one_quotient(&f_gamma, tau);
        assert_eq!(lagrange_degree2_at_tau, remainder);
        coeffs
    };
    let quotient_delta_x_commitment = coeff_form_uni_kzg_commit(srs, &vanishing_at_tau);

    (
        local_evals.multilinear_final_eval(),
        HyperKZGOpening {
            folded_oracle_commitments,
            evals_at_x: local_evals.into(),
            beta_x_commitment,
            quotient_delta_x_commitment,
        },
    )
}

#[inline(always)]
pub fn coeff_form_uni_hyperkzg_verify<E, T>(
    vk: UniKZGVerifierParams<E>,
    comm: E::G1Affine,
    alphas: &[E::Fr],
    eval: E::Fr,
    opening: &HyperKZGOpening<E>,
    fs_transcript: &mut T,
) -> bool
where
    E: MultiMillerLoop,
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
    T: Transcript<E::Fr>,
{
    opening
        .folded_oracle_commitments
        .iter()
        .for_each(|f| fs_transcript.append_u8_slice(f.to_bytes().as_ref()));

    let beta = fs_transcript.generate_challenge_field_element();
    let beta2 = beta * beta;

    let local_evals =
        HyperKZGLocalEvals::<E>::new_from_exported_evals(&opening.evals_at_x, alphas, beta);

    opening.evals_at_x.append_to_transcript(fs_transcript);

    if local_evals.multilinear_final_eval() != eval {
        return false;
    }

    let gamma = fs_transcript.generate_challenge_field_element();
    let gamma_pow_series = powers_series(&gamma, alphas.len());
    let v_beta = univariate_evaluate(&local_evals.pos_beta_evals, &gamma_pow_series);
    let v_neg_beta = univariate_evaluate(&local_evals.neg_beta_evals, &gamma_pow_series);
    let v_beta2 = univariate_evaluate(&local_evals.beta2_evals, &gamma_pow_series);
    let lagrange_degree2 =
        coeff_form_degree2_lagrange([beta, -beta, beta2], [v_beta, v_neg_beta, v_beta2]);

    let folded_g1_oracle_comms: Vec<E::G1> = opening
        .folded_oracle_commitments
        .iter()
        .map(|c| c.to_curve())
        .collect();
    let commitment_agg_g1: E::G1 =
        comm.to_curve() + univariate_evaluate(&folded_g1_oracle_comms, &gamma_pow_series[1..]);

    fs_transcript.append_u8_slice(opening.beta_x_commitment.to_bytes().as_ref());
    let tau = fs_transcript.generate_challenge_field_element();

    let q_weight = (tau - beta) * (tau - beta2) * (tau + beta);
    let lagrange_eval =
        lagrange_degree2[0] + lagrange_degree2[1] * tau + lagrange_degree2[2] * tau * tau;

    coeff_form_uni_kzg_verify(
        vk,
        (commitment_agg_g1 - opening.beta_x_commitment.to_curve() * q_weight).into(),
        tau,
        lagrange_eval,
        opening.quotient_delta_x_commitment,
    )
}
