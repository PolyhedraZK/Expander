use std::iter;

use arith::ExtensionField;
use halo2curves::{ff::Field, pairing::MultiMillerLoop, CurveAffine};
use itertools::izip;
use transcript::Transcript;

use crate::*;

//NOTE(HS): the algorithm port for HyperKZG to "HyperBiKZG" is sketched here:
// https://drive.google.com/file/d/1NcRnqdwFLcLi77DvSZH28QwslTuBVyb4/

#[inline(always)]
pub(crate) fn coeff_form_hyper_bikzg_poly_oracles_local<E, T>(
    srs: &CoefFormBiKZGLocalSRS<E>,
    coeffs: &Vec<E::Fr>,
    local_alphas: &[E::Fr],
) -> (Vec<E::G1Affine>, Vec<Vec<E::Fr>>)
where
    E: MultiMillerLoop,
    T: Transcript<E::Fr>,
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    let mut local_coeffs = coeffs.to_vec();

    local_alphas
        .iter()
        .map(|alpha| {
            local_coeffs = local_coeffs
                .chunks(2)
                .map(|c| (E::Fr::ONE - alpha) * c[0] + *alpha * c[1])
                .collect();

            let folded_oracle_commit = coeff_form_uni_kzg_commit(&srs.tau_x_srs, &local_coeffs);

            (folded_oracle_commit, local_coeffs.clone())
        })
        .unzip()
}

#[inline(always)]
pub(crate) fn coeff_form_hyper_bikzg_pm_beta_evals_local<E, T>(
    coeffs: &Vec<E::Fr>,
    folded_oracle_coeffs: &[Vec<E::Fr>],
    local_alphas: &[E::Fr],
    beta: E::Fr,
) -> (Vec<E::Fr>, Vec<E::Fr>, Vec<E::Fr>)
where
    E: MultiMillerLoop,
    T: Transcript<E::Fr>,
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

    let mut beta2_evals = vec![beta2_eval];

    let (beta_evals, neg_beta_evals): (Vec<E::Fr>, Vec<E::Fr>) =
        izip!(iter::once(coeffs).chain(folded_oracle_coeffs), local_alphas)
            .map(|(cs, alpha)| {
                let beta_eval = univariate_evaluate(cs, &beta_pow_series);
                let neg_beta_eval = univariate_evaluate(cs, &neg_beta_pow_series);

                let beta2_eval = two_inv
                    * ((beta_eval + neg_beta_eval) * (E::Fr::ONE - alpha)
                        + (beta_eval - neg_beta_eval) * beta_inv * alpha);

                beta2_evals.push(beta2_eval);

                (beta_eval, neg_beta_eval)
            })
            .unzip();

    (beta2_evals, beta_evals, neg_beta_evals)
}

#[derive(Debug, Default)]
pub(crate) struct PosNegBetaLocalEvals<E: MultiMillerLoop> {
    pub(crate) beta2_eval: E::Fr,
    pub(crate) pos_beta_evals: Vec<E::Fr>,
    pub(crate) neg_beta_evals: Vec<E::Fr>,
}

#[derive(Debug, Default)]
pub(crate) struct PosNegBetaGlobalEvals<E: MultiMillerLoop> {
    pub(crate) beta_x2_beta_y2_eval: E::Fr,
    pub(crate) beta_x2_beta_pos_y_eval: E::Fr,
    pub(crate) beta_x2_beta_neg_y_eval: E::Fr,

    pub(crate) beta_pos_x_beta_y2_evals: Vec<E::Fr>,
    pub(crate) beta_pos_x_beta_pos_y_evals: Vec<E::Fr>,
    pub(crate) beta_pos_x_beta_neg_y_evals: Vec<E::Fr>,

    pub(crate) beta_neg_x_beta_y2_evals: Vec<E::Fr>,
    pub(crate) beta_neg_x_beta_pos_y_evals: Vec<E::Fr>,
    pub(crate) beta_neg_x_beta_neg_y_evals: Vec<E::Fr>,
}

pub(crate) fn leader_evals_beta_y_interpolate<E>(
    pos_neg_beta_parties_evals: &Vec<PosNegBetaLocalEvals<E>>,
    beta: E::Fr,
    pos_neg_beta_global_evals: &mut PosNegBetaGlobalEvals<E>,
) where
    E: MultiMillerLoop,
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
}

#[inline(always)]
pub(crate) fn coeff_form_hyper_bikzg_pm_beta_evals_leader<E, T>(
    pos_neg_beta_parties_evals: &Vec<PosNegBetaLocalEvals<E>>,
    folded_oracle_coeffs: &[Vec<E::Fr>],
    mpi_alphas: &[E::Fr],
    beta: E::Fr,
) -> PosNegBetaGlobalEvals<E>
where
    E: MultiMillerLoop,
    T: Transcript<E::Fr>,
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    todo!()
}
