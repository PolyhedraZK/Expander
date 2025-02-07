use ark_std::test_rng;
use halo2curves::{
    ff::Field,
    group::{prime::PrimeCurveAffine, Curve, Group},
    msm::best_multiexp,
    pairing::{MillerLoopResult, MultiMillerLoop},
    CurveAffine,
};

use crate::{
    powers_series, univariate_degree_one_quotient, CoefFormBiKZGLocalSRS, CoefFormUniKZGSRS,
};

use super::{BiKZGProof, BiKZGVerifierParam};

#[inline(always)]
pub fn generate_coef_form_bi_kzg_local_srs_for_testing<E: MultiMillerLoop>(
    local_length: usize,
    distributed_parties: usize,
    party_rank: usize,
) -> CoefFormBiKZGLocalSRS<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    assert!(local_length.is_power_of_two());
    assert!(distributed_parties.is_power_of_two());
    assert!(party_rank < distributed_parties && 0 < party_rank);

    let mut rng = test_rng();
    let tau_x = E::Fr::random(&mut rng);
    let tau_y = E::Fr::random(&mut rng);

    let g1 = E::G1Affine::generator();

    let tau_x_geometric_progression = powers_series(&tau_x, local_length);
    let tau_y_geometric_progression = powers_series(&tau_y, distributed_parties);

    let g1_prog = g1.to_curve();
    let x_coeff_bases = {
        let mut proj_bases = vec![E::G1::identity(); local_length];
        proj_bases.iter_mut().enumerate().for_each(|(i, base)| {
            *base =
                g1_prog * tau_y_geometric_progression[party_rank] * tau_x_geometric_progression[i]
        });

        let mut g_bases = vec![E::G1Affine::default(); local_length];
        E::G1::batch_normalize(&proj_bases, &mut g_bases);

        drop(proj_bases);
        g_bases
    };

    let tau_x_srs = CoefFormUniKZGSRS::<E> {
        powers_of_tau: x_coeff_bases,
        tau_g2: (E::G2Affine::generator() * tau_x).into(),
    };

    let y_coeff_bases = {
        let mut proj_bases = vec![E::G1::identity(); distributed_parties];
        proj_bases
            .iter_mut()
            .enumerate()
            .for_each(|(i, base)| *base = g1_prog * tau_y_geometric_progression[i]);

        let mut g_bases = vec![E::G1Affine::default(); distributed_parties];
        E::G1::batch_normalize(&proj_bases, &mut g_bases);

        drop(proj_bases);
        g_bases
    };

    let tau_y_srs = CoefFormUniKZGSRS::<E> {
        powers_of_tau: y_coeff_bases,
        tau_g2: (E::G2Affine::generator() * tau_y).into(),
    };

    CoefFormBiKZGLocalSRS {
        tau_x_srs,
        tau_y_srs,
    }
}

#[inline(always)]
pub fn coeff_form_bi_kzg_open_leader<E: MultiMillerLoop>(
    srs: &CoefFormBiKZGLocalSRS<E>,
    evals_and_opens: &[(E::Fr, E::G1)],
    beta: E::Fr,
    eval: E::Fr,
) -> BiKZGProof<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    assert_eq!(srs.tau_y_srs.powers_of_tau.len(), evals_and_opens.len());

    let x_open: E::G1 = evals_and_opens.iter().map(|(_, o)| o).sum();
    let gammas: Vec<E::Fr> = evals_and_opens.iter().map(|(e, _)| *e).collect();

    let (div, remainder) = univariate_degree_one_quotient(&gammas, beta);
    assert_eq!(remainder, eval);

    let y_open = best_multiexp(&div, &srs.tau_y_srs.powers_of_tau[..div.len()]);

    BiKZGProof {
        quotient_x: x_open.into(),
        quotient_y: y_open.into(),
    }
}

#[inline(always)]
pub fn coeff_form_bi_kzg_verify<E: MultiMillerLoop>(
    vk: BiKZGVerifierParam<E>,
    comm: E::G1,
    alpha: E::Fr,
    beta: E::Fr,
    eval: E::Fr,
    opening: BiKZGProof<E>,
) -> bool
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    let g1_eval: E::G1Affine = (E::G1Affine::generator() * eval).into();
    let g2_alpha: E::G2 = E::G2Affine::generator() * alpha;
    let g2_beta: E::G2 = E::G2Affine::generator() * beta;

    let gt_result = E::multi_miller_loop(&[
        (
            &opening.quotient_x,
            &(vk.tau_x_g2.to_curve() - g2_alpha).to_affine().into(),
        ),
        (
            &opening.quotient_y,
            &(vk.tau_y_g2.to_curve() - g2_beta).to_affine().into(),
        ),
        (
            &(g1_eval - comm.to_affine()).into(),
            &E::G2Affine::generator().into(),
        ),
    ]);

    gt_result.final_exponentiation().is_identity().into()
}
