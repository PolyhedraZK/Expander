use halo2curves::{
    ff::Field,
    group::{prime::PrimeCurveAffine, Curve, Group},
    msm,
    pairing::{MillerLoopResult, MultiMillerLoop},
    CurveAffine,
};
use itertools::izip;

use crate::*;

#[inline(always)]
pub fn generate_coef_form_uni_kzg_srs_for_testing<E: MultiMillerLoop>(
    length: usize,
    mut rng: impl rand::RngCore,
) -> CoefFormUniKZGSRS<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    assert!(length.is_power_of_two());

    let tau = E::Fr::random(&mut rng);
    let g1 = E::G1Affine::generator();

    let tau_geometric_progression = powers_series(&tau, length);

    let g1_prog = g1.to_curve();
    let coeff_bases = {
        let mut proj_bases = vec![g1_prog; length];
        izip!(&mut proj_bases, &tau_geometric_progression).for_each(|(b, tau_i)| *b *= *tau_i);

        let mut g_bases = vec![E::G1Affine::default(); length];
        E::G1::batch_normalize(&proj_bases, &mut g_bases);

        drop(proj_bases);
        g_bases
    };

    CoefFormUniKZGSRS {
        powers_of_tau: coeff_bases,
        tau_g2: (E::G2Affine::generator() * tau).into(),
    }
}

#[inline(always)]
pub fn coeff_form_uni_kzg_commit<E: MultiMillerLoop>(
    srs: &CoefFormUniKZGSRS<E>,
    coeffs: &[E::Fr],
) -> E::G1Affine
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    assert!(srs.powers_of_tau.len() >= coeffs.len());

    let mut com = E::G1::generator() * E::Fr::ZERO;
    msm::multiexp_serial(coeffs, &srs.powers_of_tau[..coeffs.len()], &mut com);
    com.into()
}

#[inline(always)]
pub fn coeff_form_uni_kzg_open_eval<E: MultiMillerLoop>(
    srs: &CoefFormUniKZGSRS<E>,
    coeffs: &[E::Fr],
    alpha: E::Fr,
) -> (E::Fr, E::G1Affine)
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    assert!(srs.powers_of_tau.len() >= coeffs.len());

    let (div, eval) = univariate_degree_one_quotient(coeffs, alpha);
    let mut opening = E::G1::generator() * E::Fr::ZERO;
    msm::multiexp_serial(&div, &srs.powers_of_tau[..div.len()], &mut opening);

    (eval, opening.into())
}

#[inline(always)]
pub fn coeff_form_uni_kzg_verify<E: MultiMillerLoop>(
    vk: UniKZGVerifierParams<E>,
    comm: E::G1Affine,
    alpha: E::Fr,
    eval: E::Fr,
    opening: E::G1Affine,
) -> bool
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    let g1_eval: E::G1Affine = (E::G1Affine::generator() * eval).into();
    let g2_alpha: E::G2 = E::G2Affine::generator() * alpha;

    let gt_result = E::multi_miller_loop(&[
        (
            &opening,
            &(vk.tau_g2.to_curve() - g2_alpha).to_affine().into(),
        ),
        (&(g1_eval - comm).into(), &E::G2Affine::generator().into()),
    ]);

    gt_result.final_exponentiation().is_identity().into()
}
