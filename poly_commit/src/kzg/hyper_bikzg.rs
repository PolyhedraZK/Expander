use arith::ExtensionField;
use halo2curves::{pairing::MultiMillerLoop, CurveAffine};

use crate::*;

//NOTE(HS): the algorithm port for HyperKZG to "HyperBiKZG" is sketched here:
// https://drive.google.com/file/d/1NcRnqdwFLcLi77DvSZH28QwslTuBVyb4/

#[allow(unused)]
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

#[allow(unused)]
pub(crate) fn leader_evals_beta_y_interpolate<E>(
    pos_neg_beta_parties_evals: &[HyperKZGExportedLocalEvals<E>],
    beta: E::Fr,
    pos_neg_beta_global_evals: &mut PosNegBetaGlobalEvals<E>,
) where
    E: MultiMillerLoop,
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    todo!()
}

#[allow(unused)]
#[inline(always)]
pub(crate) fn coeff_form_hyper_bikzg_pm_beta_evals_leader<E>(
    pos_neg_beta_parties_evals: &[HyperKZGExportedLocalEvals<E>],
    folded_oracle_coeffs: &[Vec<E::Fr>],
    mpi_alphas: &[E::Fr],
    beta: E::Fr,
) -> PosNegBetaGlobalEvals<E>
where
    E: MultiMillerLoop,
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    todo!()
}
