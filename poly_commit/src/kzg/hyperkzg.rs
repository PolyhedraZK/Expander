use std::iter;

use arith::ExtensionField;
use halo2curves::{ff::Field, group::GroupEncoding, pairing::MultiMillerLoop, CurveAffine};
use itertools::izip;
use transcript::Transcript;

use crate::{
    coeff_form_uni_kzg_commit, even_odd_coeffs_separate, powers_of_field_elements,
    univariate_degree_one_quotient, univariate_evaluate,
};

use super::{CoefFormUniKZGSRS, HyperKZGOpening};

pub fn coeff_form_uni_hyperkzg_open<E: MultiMillerLoop, T: Transcript<E::Fr>>(
    srs: &CoefFormUniKZGSRS<E>,
    coeffs: &Vec<E::Fr>,
    alphas: &[E::Fr],
    _eval: E::Fr,
    fs_transcript: &mut T,
) -> HyperKZGOpening<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    let mut local_coeffs = coeffs.to_vec();

    let (folded_oracle_commits, folded_oracle_coeffs): (Vec<E::G1>, Vec<Vec<E::Fr>>) = alphas
        [..alphas.len() - 1]
        .iter()
        .map(|alpha| {
            let (evens, odds) = even_odd_coeffs_separate(&local_coeffs);

            local_coeffs = izip!(&evens, &odds)
                .map(|(e, o)| (E::Fr::ONE - alpha) * e + *alpha * *o)
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
    let beta_pow_series = powers_of_field_elements(&beta, coeffs.len());
    let neg_beta_pow_series = powers_of_field_elements(&(-beta), coeffs.len());

    let beta2_eval = {
        let beta2_pow_series = powers_of_field_elements(&beta2, coeffs.len());
        univariate_evaluate(coeffs, &beta2_pow_series)
    };

    fs_transcript.append_field_element(&beta2_eval);
    let mut beta2_evals = vec![beta2_eval];

    let (beta_evals, neg_beta_evals): (Vec<E::Fr>, Vec<E::Fr>) = izip!(
        iter::once(coeffs).chain(folded_oracle_coeffs.iter()),
        alphas
    )
    .enumerate()
    .map(|(i, (cs, alpha))| {
        let beta_eval = univariate_evaluate(cs, &beta_pow_series);
        let neg_beta_eval = univariate_evaluate(cs, &neg_beta_pow_series);

        if i < alphas.len() - 1 {
            let beta2_eval = (beta_eval + neg_beta_eval) * two_inv * (E::Fr::ONE - alpha)
                + (beta_eval - neg_beta_eval) * two_inv * beta_inv * alpha;

            beta2_evals.push(beta2_eval);
        }

        fs_transcript.append_field_element(&beta_eval);
        fs_transcript.append_field_element(&neg_beta_eval);

        (beta_eval, neg_beta_eval)
    })
    .unzip();

    let gamma = fs_transcript.generate_challenge_field_element();
    let gamma_pow_series = powers_of_field_elements(&gamma, alphas.len());
    let v_beta = univariate_evaluate(&beta_evals, &gamma_pow_series);
    let v_neg_beta = univariate_evaluate(&neg_beta_evals, &gamma_pow_series);
    let v_beta2 = univariate_evaluate(&beta2_evals, &gamma_pow_series);
    let f_gamma = {
        let mut f = coeffs.clone();
        izip!(&gamma_pow_series[1..], &folded_oracle_coeffs).for_each(|(gamma_i, folded_f)| {
            izip!(&mut f, folded_f).for_each(|(f_i, folded_f_i)| {
                *f_i += *folded_f_i * *gamma_i;
            });
        });
        f
    };
    let lagrange_degree2: Vec<E::Fr> = {
        let l_beta_nom = [-beta2 * beta, -beta2 + beta, E::Fr::ONE];
        let l_beta_denom_inv: E::Fr = ((beta - beta2) * (beta + beta)).invert().unwrap();
        let l_beta: Vec<E::Fr> = l_beta_nom.iter().map(|i| *i * l_beta_denom_inv).collect();

        let l_neg_beta_norm = [beta2 * beta, -beta2 - beta, E::Fr::ONE];
        let l_neg_beta_denom_inv: E::Fr = ((beta2 - beta) * (beta2 + beta)).invert().unwrap();
        let l_neg_beta: Vec<E::Fr> = l_neg_beta_norm
            .iter()
            .map(|i| *i * l_neg_beta_denom_inv)
            .collect();

        let l_beta2_norm = [-beta2, E::Fr::ZERO, E::Fr::ONE];
        let l_beta2_denom_inv: E::Fr = ((beta + beta2) * (beta + beta)).invert().unwrap();
        let l_beta2: Vec<E::Fr> = l_beta2_norm
            .iter()
            .map(|i| *i * l_beta2_denom_inv)
            .collect();

        izip!(l_beta, l_neg_beta, l_beta2)
            .map(|(l_beta_i, l_neg_beta_i, l_beta2_i)| {
                l_beta_i * v_beta + l_neg_beta_i * v_neg_beta + l_beta2_i * v_beta2
            })
            .collect()
    };
    let f_gamma_quotient = {
        let mut nom = f_gamma.clone();
        izip!(&mut nom, &lagrange_degree2).for_each(|(n, l)| *n -= l);

        let (nom_1, remainder_1) = univariate_degree_one_quotient(&nom, beta);
        assert_eq!(remainder_1, E::Fr::ZERO);

        let (nom_2, remainder_2) = univariate_degree_one_quotient(&nom_1, beta2);
        assert_eq!(remainder_2, E::Fr::ZERO);

        let (nom_3, remainder_3) = univariate_degree_one_quotient(&nom_2, -beta);
        assert_eq!(remainder_3, E::Fr::ZERO);

        nom_3
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
        izip!(&mut poly, &f_gamma_quotient).for_each(|(p, f)| *p -= *f * f_gamma_denom);

        let (quotient, remainder) = univariate_degree_one_quotient(&poly, tau);
        assert_eq!(remainder, E::Fr::ZERO);

        quotient
    };
    let vanishing_at_tau_commitment = coeff_form_uni_kzg_commit(srs, &vanishing_at_tau);

    HyperKZGOpening {
        folded_oracle_commitments: folded_oracle_commits,
        f_beta2: beta2_eval,
        evals_at_beta: beta_evals,
        evals_at_neg_beta: neg_beta_evals,
        beta_commitment: f_gamma_quotient_com,
        tau_vanishing_commitment: vanishing_at_tau_commitment,
    }
}
