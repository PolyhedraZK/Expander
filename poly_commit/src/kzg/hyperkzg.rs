use arith::ExtensionField;
use halo2curves::{ff::Field, group::Group, pairing::MultiMillerLoop, CurveAffine};
use transcript::Transcript;

use crate::{
    coeff_form_uni_kzg_commit, coeff_form_uni_kzg_open, even_odd_coeffs_separate, merge_coeffs,
    powers_of_field_elements, univariate_evaluate,
};

use super::CoefFormUniKZGSRS;

pub fn coeff_form_uni_hyperkzg_open<E: MultiMillerLoop, T: Transcript<E::Fr>>(
    srs: &CoefFormUniKZGSRS<E>,
    coeffs: &[E::Fr],
    alphas: &[E::Fr],
    _eval: E::Fr,
    beta: E::Fr,
    gamma: E::Fr,
    _transcript: &mut T,
) where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    let beta2 = beta * beta;
    let power_series = powers_of_field_elements(&beta2, coeffs.len() >> 1);
    let mut local_coeffs = coeffs.to_vec();
    let mut combine_weight = E::Fr::ONE;

    let mut folded_commitment_agg: E::G1 = E::G1::generator() * E::Fr::ZERO;
    let mut odd_commitment_agg: E::G1 = E::G1::generator() * E::Fr::ZERO;
    let mut even_commitment_agg: E::G1 = E::G1::generator() * E::Fr::ZERO;

    let mut total_opening_agg: E::G1 = E::G1::generator() * E::Fr::ZERO;
    let mut odd_opening_agg: E::G1 = E::G1::generator() * E::Fr::ZERO;
    let mut even_opening_agg: E::G1 = E::G1::generator() * E::Fr::ZERO;

    let mut odd_eval_agg: E::Fr = E::Fr::ZERO;
    let mut even_eval_agg: E::Fr = E::Fr::ZERO;

    alphas.iter().enumerate().for_each(|(i, alpha)| {
        if i != 0 {
            let local_com = coeff_form_uni_kzg_commit(srs, &local_coeffs);
            folded_commitment_agg += local_com * combine_weight;
        }

        let (evens, odds) = even_odd_coeffs_separate(&local_coeffs);

        let local_evens_com = coeff_form_uni_kzg_commit(srs, &evens);
        even_commitment_agg += local_evens_com * combine_weight;

        let local_odds_com = coeff_form_uni_kzg_commit(srs, &odds);
        odd_commitment_agg += local_odds_com * combine_weight;

        let evens_beta2_eval = univariate_evaluate(&evens, &power_series);
        let local_even_opening = coeff_form_uni_kzg_open(srs, &evens, beta2, evens_beta2_eval);
        even_opening_agg += local_even_opening * combine_weight;
        even_eval_agg += evens_beta2_eval * combine_weight;

        let odds_beta2_eval = univariate_evaluate(&odds, &power_series);
        let local_odd_opening = coeff_form_uni_kzg_open(srs, &odds, beta2, odds_beta2_eval);
        odd_opening_agg += local_odd_opening * combine_weight;
        odd_eval_agg += odds_beta2_eval * combine_weight;

        let current_eval = evens_beta2_eval + beta * odds_beta2_eval;
        let local_total_opening = coeff_form_uni_kzg_open(srs, &local_coeffs, beta, current_eval);
        total_opening_agg += local_total_opening * combine_weight;

        local_coeffs = merge_coeffs(evens, odds, *alpha);
        combine_weight *= gamma;
    });

    todo!()
}
