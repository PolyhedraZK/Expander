use ark_ec::AffineRepr;
use ark_ec::CurveGroup;
use ark_ec::{pairing::Pairing, VariableBaseMSM};
use ark_std::rand::RngCore;
use ark_std::One;
use ark_std::UniformRand;
use rayon::prelude::*;
use serdes::ExpSerde;

use crate::*;

#[inline(always)]
pub(crate) fn generate_coef_form_uni_kzg_srs_for_testing<E: Pairing>(
    length: usize,
    mut rng: impl RngCore,
) -> CoefFormUniKZGSRS<E>
where
    E::G1Affine: ExpSerde,
    E::G2Affine: ExpSerde,
{
    assert!(length.is_power_of_two());

    let tau = E::ScalarField::rand(&mut rng);
    let g1 = E::G1Affine::generator();

    let tau_geometric_progression = powers_series(&tau, length);

    let g1_prog: E::G1 = g1.into();
    let coeff_bases = {
        let mut proj_bases = vec![g1_prog; length];
        proj_bases
            .par_iter_mut()
            .zip(tau_geometric_progression.par_iter())
            .for_each(|(b, tau_i)| *b *= tau_i);

        let g_bases = E::G1::normalize_batch(&proj_bases);

        drop(proj_bases);
        g_bases
    };

    CoefFormUniKZGSRS {
        powers_of_tau: coeff_bases,
        tau_g2: (E::G2Affine::generator() * tau).into(),
    }
}

#[inline(always)]
pub(crate) fn coeff_form_uni_kzg_commit<E: Pairing>(
    srs: &CoefFormUniKZGSRS<E>,
    coeffs: &[E::ScalarField],
) -> E::G1Affine
where
    E::G1Affine: ExpSerde,
    E::G2Affine: ExpSerde,
{
    assert!(srs.powers_of_tau.len() >= coeffs.len());

    let com: E::G1 = VariableBaseMSM::msm(&srs.powers_of_tau[..coeffs.len()], coeffs).unwrap();

    com.into()
}

#[inline(always)]
pub fn coeff_form_uni_kzg_open_eval<E: Pairing>(
    srs: &CoefFormUniKZGSRS<E>,
    coeffs: &[E::ScalarField],
    alpha: E::ScalarField,
) -> (E::ScalarField, E::G1Affine)
where
    E::G1Affine: ExpSerde,
    E::G2Affine: ExpSerde,
{
    assert!(srs.powers_of_tau.len() >= coeffs.len());

    let (div, eval) = univariate_degree_one_quotient(coeffs, alpha);
    let opening: E::G1 = VariableBaseMSM::msm(&srs.powers_of_tau[..div.len()], &div).unwrap();

    (eval, opening.into())
}

#[inline(always)]
pub(crate) fn coeff_form_uni_kzg_verify<E: Pairing>(
    vk: &UniKZGVerifierParams<E>,
    comm: E::G1Affine,
    alpha: E::ScalarField,
    eval: E::ScalarField,
    opening: E::G1Affine,
) -> bool
where
    E::G2Affine: ExpSerde,
{
    let g1_eval: E::G1Affine = (E::G1Affine::generator() * eval).into();
    let g2_alpha: E::G2 = E::G2Affine::generator() * alpha;

    let gt_result = E::multi_miller_loop(
        &[opening, (g1_eval - comm).into()],
        &[
            (vk.tau_g2 - g2_alpha).into(),
            E::G2Affine::generator().into(),
        ],
    );

    E::final_exponentiation(gt_result).unwrap().0 == E::TargetField::one()
}

#[cfg(test)]
mod tests {
    use arith::Fr;
    use ark_bn254::Bn254;
    use ark_std::test_rng;

    use crate::*;

    #[test]
    fn test_coefficient_form_univariate_kzg_e2e() {
        // \prod_{i \in [1, 7]} (x + i)
        let poly = vec![
            Fr::from(5040u32),
            Fr::from(13068u64),
            Fr::from(13132u64),
            Fr::from(6769u64),
            Fr::from(1960u64),
            Fr::from(322u64),
            Fr::from(28u64),
            Fr::from(1u64),
        ];
        let alpha = Fr::from(3u64);
        let eval = Fr::from(604800u64);

        let mut rng = test_rng();
        let srs = generate_coef_form_uni_kzg_srs_for_testing::<Bn254>(8, &mut rng);
        let vk: UniKZGVerifierParams<Bn254> = From::from(&srs);
        let com = coeff_form_uni_kzg_commit(&srs, &poly);

        let (actual_eval, opening) = coeff_form_uni_kzg_open_eval(&srs, &poly, alpha);
        assert_eq!(actual_eval, eval);

        assert!(coeff_form_uni_kzg_verify(&vk, com, alpha, eval, opening))
    }

    #[test]
    fn test_coefficient_form_univariate_kzg_constant_e2e() {
        let poly = vec![Fr::from(100u64)];
        let alpha = Fr::from(3u64);
        let eval = Fr::from(100u64);

        let mut rng = test_rng();
        let srs = generate_coef_form_uni_kzg_srs_for_testing::<Bn254>(8, &mut rng);
        let vk: UniKZGVerifierParams<Bn254> = From::from(&srs);
        let com = coeff_form_uni_kzg_commit(&srs, &poly);

        let (actual_eval, opening) = coeff_form_uni_kzg_open_eval(&srs, &poly, alpha);
        assert_eq!(actual_eval, eval);

        assert!(coeff_form_uni_kzg_verify(&vk, com, alpha, eval, opening))
    }
}
