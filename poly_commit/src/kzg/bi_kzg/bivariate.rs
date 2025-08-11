use ark_ec::AffineRepr;
use ark_ec::VariableBaseMSM;
use ark_ec::{pairing::Pairing, CurveGroup};
use ark_std::One;
use ark_std::UniformRand;
use itertools::izip;
use serdes::ExpSerde;

use crate::*;

#[inline(always)]
pub fn generate_coef_form_bi_kzg_local_srs_for_testing<E: Pairing>(
    local_length: usize,
    distributed_parties: usize,
    party_rank: usize,
    mut rng: impl rand::RngCore,
) -> CoefFormBiKZGLocalSRS<E>
where
    E::G1Affine: ExpSerde,
    E::G2Affine: ExpSerde,
{
    assert!(local_length.is_power_of_two());
    assert!(distributed_parties.is_power_of_two());
    assert!(party_rank < distributed_parties);

    let tau_x = E::ScalarField::rand(&mut rng);
    let tau_y = E::ScalarField::rand(&mut rng);

    let g1 = E::G1Affine::generator();

    let tau_x_geometric_progression = powers_series(&tau_x, local_length);
    let tau_y_geometric_progression = powers_series(&tau_y, distributed_parties);

    let g1_prog: E::G1 = g1.into();
    let x_coeff_bases = {
        let mut proj_bases = vec![g1_prog * tau_y_geometric_progression[party_rank]; local_length];
        izip!(&mut proj_bases, &tau_x_geometric_progression).for_each(|(b, tau_xi)| *b *= *tau_xi);
        let g_bases = <E::G1 as CurveGroup>::normalize_batch(&proj_bases);

        drop(proj_bases);
        g_bases
    };

    let tau_x_srs = CoefFormUniKZGSRS::<E> {
        powers_of_tau: x_coeff_bases,
        tau_g2: (E::G2Affine::generator() * tau_x).into(),
    };

    let y_coeff_bases = {
        let mut proj_bases = vec![g1_prog; distributed_parties];
        izip!(&mut proj_bases, &tau_y_geometric_progression).for_each(|(b, tau_yi)| *b *= *tau_yi);

        let g_bases = <E::G1 as CurveGroup>::normalize_batch(&proj_bases);

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
pub fn coeff_form_bi_kzg_open_leader<E: Pairing>(
    srs: &CoefFormBiKZGLocalSRS<E>,
    evals_and_opens: &[(E::ScalarField, E::G1Affine)],
    beta: E::ScalarField,
) -> (E::ScalarField, BiKZGProof<E>)
where
    E::G1Affine: ExpSerde,
    E::G2Affine: ExpSerde,
{
    assert_eq!(srs.tau_y_srs.powers_of_tau.len(), evals_and_opens.len());

    let x_open: E::G1 = evals_and_opens.iter().map(|(_, o)| E::G1::from(*o)).sum();
    let gammas: Vec<E::ScalarField> = evals_and_opens.iter().map(|(e, _)| *e).collect();

    let (div, eval) = univariate_degree_one_quotient(&gammas, beta);

    let y_open: E::G1 =
        VariableBaseMSM::msm(&srs.tau_y_srs.powers_of_tau[..div.len()], &div).unwrap();

    (
        eval,
        BiKZGProof {
            quotient_x: x_open.into(),
            quotient_y: y_open.into(),
        },
    )
}

#[inline(always)]
pub fn coeff_form_bi_kzg_verify<E: Pairing>(
    vk: BiKZGVerifierParam<E>,
    comm: E::G1Affine,
    alpha: E::ScalarField,
    beta: E::ScalarField,
    eval: E::ScalarField,
    opening: BiKZGProof<E>,
) -> bool
where
    E::G1Affine: ExpSerde,
    E::G2Affine: ExpSerde,
{
    let g1_eval: E::G1Affine = (E::G1Affine::generator() * eval).into();
    let g2_alpha: E::G2 = E::G2Affine::generator() * alpha;
    let g2_beta: E::G2 = E::G2Affine::generator() * beta;

    let gt_result = E::multi_miller_loop(
        &[
            opening.quotient_x,
            opening.quotient_y,
            (g1_eval - comm).into(),
        ],
        &[
            (vk.tau_x_g2 - g2_alpha).into(),
            (vk.tau_y_g2 - g2_beta).into(),
            E::G2Affine::generator().into(),
        ],
    );
    E::final_exponentiation(gt_result).unwrap().0 == E::TargetField::one()
}

#[cfg(test)]
mod tests {
    use arith::Fr;
    use ark_bn254::{Bn254, G1Affine, G1Projective};
    use ark_ec::{AffineRepr, CurveGroup};
    use ark_ff::UniformRand;
    use ark_std::test_rng;
    use itertools::izip;

    use crate::*;

    #[test]
    fn test_coefficient_form_bivariate_kzg_e2e() {
        let x_degree = 15;
        let y_degree = 7;

        let party_srs: Vec<CoefFormBiKZGLocalSRS<Bn254>> = (0..=y_degree)
            .map(|rank| {
                let mut rng = test_rng();
                generate_coef_form_bi_kzg_local_srs_for_testing(
                    x_degree + 1,
                    y_degree + 1,
                    rank,
                    &mut rng,
                )
            })
            .collect();

        let mut rng = test_rng();
        let xy_coeffs: Vec<Vec<Fr>> = (0..=y_degree)
            .map(|_| (0..=x_degree).map(|_| Fr::rand(&mut rng)).collect())
            .collect();

        let commitments: Vec<_> = izip!(&party_srs, &xy_coeffs)
            .map(|(srs, x_coeffs)| coeff_form_uni_kzg_commit(&srs.tau_x_srs, x_coeffs))
            .collect();

        let global_commitment_g1: G1Projective = commitments.iter().map(|c| c.into_group()).sum::<G1Projective>();
        let global_commitment: G1Affine = global_commitment_g1.into_affine();

        let alpha = Fr::rand(&mut rng);
        let evals_and_opens: Vec<(Fr, G1Affine)> = izip!(&party_srs, &xy_coeffs)
            .map(|(srs, x_coeffs)| coeff_form_uni_kzg_open_eval(&srs.tau_x_srs, x_coeffs, alpha))
            .collect();

        let beta = Fr::rand(&mut rng);
        let (final_eval, final_opening) =
            coeff_form_bi_kzg_open_leader(&party_srs[0], &evals_and_opens, beta);

        let vk: BiKZGVerifierParam<Bn254> = From::from(&party_srs[0]);

        assert!(coeff_form_bi_kzg_verify(
            vk,
            global_commitment,
            alpha,
            beta,
            final_eval,
            final_opening,
        ));
    }
}
