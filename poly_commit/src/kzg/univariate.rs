use ark_std::test_rng;
use halo2curves::{
    ff::Field,
    group::{prime::PrimeCurveAffine, Curve, Group},
    msm::best_multiexp,
    pairing::{MillerLoopResult, MultiMillerLoop},
    CurveAffine,
};

use crate::{powers_of_field_elements, univariate_degree_one_quotient};

use super::{CoefFormUniKZGSRS, UniKZGCommitment, UniKZGOpening, UniKZGVerifierParams};

pub fn generate_srs_for_testing<E: MultiMillerLoop>(length: usize) -> CoefFormUniKZGSRS<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    assert!(length.is_power_of_two());

    let mut rng = test_rng();
    let tau = E::Fr::random(&mut rng);
    let g1 = E::G1Affine::generator();

    let tau_geometric_progression = powers_of_field_elements(&tau, length);

    let g1_prog = g1.to_curve();
    let coeff_bases = {
        let mut proj_bases = vec![E::G1::identity(); length];
        proj_bases
            .iter_mut()
            .enumerate()
            .for_each(|(i, base)| *base = g1_prog * tau_geometric_progression[i]);

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

pub fn coeff_form_uni_kzg_commit<E: MultiMillerLoop>(
    srs: &CoefFormUniKZGSRS<E>,
    coeffs: &[E::Fr],
) -> UniKZGCommitment<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    assert_eq!(srs.powers_of_tau.len(), coeffs.len());

    let com = best_multiexp(coeffs, srs.powers_of_tau.as_slice());

    UniKZGCommitment { com: com.into() }
}

pub fn coeff_form_uni_kzg_opening<E: MultiMillerLoop>(
    srs: &CoefFormUniKZGSRS<E>,
    coeffs: &[E::Fr],
    alpha: E::Fr,
    eval: E::Fr,
) -> UniKZGOpening<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    assert_eq!(srs.powers_of_tau.len(), coeffs.len());

    let (div, remainder) = univariate_degree_one_quotient(coeffs, alpha);
    assert_eq!(remainder, eval);

    let opening = best_multiexp(&div, srs.powers_of_tau.as_slice());

    UniKZGOpening {
        opening: opening.into(),
    }
}

pub fn coeff_form_uni_kzg_verify<E: MultiMillerLoop>(
    vk: UniKZGVerifierParams<E>,
    comm: UniKZGCommitment<E>,
    alpha: E::Fr,
    eval: E::Fr,
    opening: UniKZGOpening<E>,
) -> bool
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    let g1_eval: E::G1Affine = (E::G1Affine::generator() * eval).into();
    let g2_alpha: E::G2 = E::G2Affine::generator() * alpha;

    let gt_result = E::multi_miller_loop(&[
        (
            &opening.opening,
            &(vk.tau_g2.to_curve() - g2_alpha).to_affine().into(),
        ),
        (
            &(g1_eval - comm.com).into(),
            &E::G2Affine::generator().into(),
        ),
    ]);

    gt_result.final_exponentiation().is_identity().into()
}
