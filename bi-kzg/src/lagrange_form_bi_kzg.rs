use std::{borrow::Borrow, marker::PhantomData};

use ark_std::{end_timer, start_timer};
use halo2curves::ff::Field;
use halo2curves::group::prime::PrimeCurveAffine;
use halo2curves::group::Curve;
use halo2curves::group::Group;
use halo2curves::msm::best_multiexp;
use halo2curves::pairing::{MillerLoopResult, MultiMillerLoop};
use halo2curves::CurveAffine;
use itertools::Itertools;
use rand::RngCore;

use crate::parallelize;
use crate::poly::{lagrange_coefficients, BivariateLagrangePolynomial};
use crate::primitive_root_of_unity;
use crate::{
    pcs::PolynomialCommitmentScheme,
    util::{powers_of_field_elements, tensor_product_parallel},
    BiKZGCommitment, BiKZGProof, BiKZGVerifierParam, LagrangeFormBiKZGSRS,
};

/// Commit to the bi-variate polynomial in its lagrange form.
/// this should be the preferred form for commitment.
pub struct LagrangeFormBiKZG<E: MultiMillerLoop> {
    _phantom: PhantomData<E>,
}

impl<E: MultiMillerLoop> PolynomialCommitmentScheme for LagrangeFormBiKZG<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    type SRS = LagrangeFormBiKZGSRS<E>;
    type ProverParam = LagrangeFormBiKZGSRS<E>;
    type VerifierParam = BiKZGVerifierParam<E>;
    type Polynomial = BivariateLagrangePolynomial<E::Fr>;
    type Commitment = BiKZGCommitment<E>;
    type Proof = BiKZGProof<E>;
    type Evaluation = E::Fr;
    type Point = (E::Fr, E::Fr);
    type BatchProof = Vec<Self::Proof>;

    fn gen_srs_for_testing(
        mut rng: impl RngCore,
        supported_n: usize,
        supported_m: usize,
    ) -> Self::SRS {
        assert!(supported_n.is_power_of_two());
        assert!(supported_m.is_power_of_two());

        let tau_0 = E::Fr::random(&mut rng);
        let tau_1 = E::Fr::random(&mut rng);

        let g1 = E::G1Affine::generator();

        // roots of unity for supported_n and supported_m
        let omega_0 = primitive_root_of_unity(supported_n);
        let omega_1 = primitive_root_of_unity(supported_m);

        // computes the vector of L_i^N(tau_0) * L_j^M(tau_1)
        // for i in 0..supported_n and j in 0..supported_m
        let (lagrange_tau_0, lagrange_scalars) = {
            let powers_of_omega_0 = powers_of_field_elements(&omega_0, supported_n);
            let lagrange_tau_0 = lagrange_coefficients(&powers_of_omega_0, &tau_0);
            let powers_of_omega_1 = powers_of_field_elements(&omega_1, supported_m);
            let lagrange_tau_1 = lagrange_coefficients(&powers_of_omega_1, &tau_1);
            let lagrange_scalars = tensor_product_parallel(&lagrange_tau_0, &lagrange_tau_1);

            (lagrange_tau_0, lagrange_scalars)
        };

        let g1_prog = g1.to_curve();
        let lagrange_x_bases = {
            let mut proj_bases = vec![E::G1::identity(); supported_n];
            parallelize(&mut proj_bases, |g, start| {
                for (idx, g) in g.iter_mut().enumerate() {
                    let offset = start + idx;
                    *g = g1_prog * lagrange_tau_0[offset];
                }
            });

            let mut g_bases = vec![E::G1Affine::identity(); supported_n];
            parallelize(&mut g_bases, |g, start| {
                E::G1::batch_normalize(&proj_bases[start..start + g.len()], g);
            });
            drop(proj_bases);
            g_bases
        };

        let lagrange_bases = {
            let mut proj_bases = vec![E::G1::identity(); supported_n * supported_m];
            parallelize(&mut proj_bases, |g, start| {
                for (idx, g) in g.iter_mut().enumerate() {
                    let offset = start + idx;
                    *g = g1 * lagrange_scalars[offset];
                }
            });

            let mut affine_bases = vec![E::G1Affine::identity(); supported_n * supported_m];
            parallelize(&mut affine_bases, |g, start| {
                E::G1::batch_normalize(&proj_bases[start..start + g.len()], g);
            });
            drop(proj_bases);
            affine_bases
        };

        LagrangeFormBiKZGSRS {
            g: g1,
            powers_of_g_lagrange_over_x: lagrange_x_bases,
            powers_of_g_lagrange_over_both_roots: lagrange_bases,
            h: E::G2Affine::generator(),
            tau_0_h: (E::G2Affine::generator() * tau_0).into(),
            tau_1_h: (E::G2Affine::generator() * tau_1).into(),
        }
    }

    // fn trim(
    //     srs: impl Borrow<Self::SRS>,
    //     supported_degree: Option<usize>,
    //     supported_num_vars: Option<usize>,
    // ) -> (Self::ProverParam, Self::VerifierParam) {
    //     unimplemented!()
    // }

    fn commit(
        prover_param: impl Borrow<Self::ProverParam>,
        poly: &Self::Polynomial,
    ) -> Self::Commitment {
        let timer = start_timer!(|| format!(
            "Committing to lagrange polynomial of degree {} {}",
            poly.degree_0, poly.degree_1
        ));

        let com = best_multiexp(
            &poly.coefficients,
            prover_param
                .borrow()
                .powers_of_g_lagrange_over_both_roots
                .as_slice(),
        );

        end_timer!(timer);

        Self::Commitment { com: com.into() }
    }

    fn open(
        prover_param: impl Borrow<Self::ProverParam>,
        polynomial: &Self::Polynomial,
        point: &Self::Point,
    ) -> (Self::Proof, Self::Evaluation) {
        let timer = start_timer!(|| format!(
            "Opening polynomial of degree {} {}",
            polynomial.degree_0, polynomial.degree_1
        ));

        let a = point.0;
        let b = point.1;
        let u = polynomial.evaluate(&a, &b);

        let timer2 = start_timer!(|| "Computing the proof pi0");
        let (pi_0, f_x_b) = {
            let f_x_b = polynomial.evaluate_at_y(&point.1);

            let omega_0 = primitive_root_of_unity(polynomial.degree_0);
            let powers_of_omega_0 =
                powers_of_field_elements::<E::Fr>(&omega_0, polynomial.degree_0);
            // todo use batch inversion
            let powers_of_omega_0_minus_x_inv = powers_of_omega_0
                .iter()
                .map(|w| (*w - point.0).invert().unwrap())
                .collect::<Vec<_>>();

            let q_0_x_b = f_x_b
                .iter()
                .zip(powers_of_omega_0_minus_x_inv)
                .map(|(v0, v1)| (*v0 - u) * v1)
                .collect::<Vec<_>>();

            let pi_0 = best_multiexp(
                &q_0_x_b,
                prover_param.borrow().powers_of_g_lagrange_over_x.as_ref(),
            )
            .to_affine();
            (pi_0, f_x_b)
        };
        end_timer!(timer2);

        // f(X, Y) = qx(X)(X - a) + qy(X, Y)(Y - b) + u
        let timer2 = start_timer!(|| "Computing the proof pi1");
        let pi_1 = {
            let omega_1 = primitive_root_of_unity(polynomial.degree_1);
            let powers_of_omega_1 =
                powers_of_field_elements::<E::Fr>(&omega_1, polynomial.degree_1);

            // todo use batch inversion
            let q_1_x_y = polynomial
                .coefficients
                .chunks_exact(polynomial.degree_0)
                .zip_eq(powers_of_omega_1)
                .flat_map(|(coeffs_i, w_y_i)| {
                    coeffs_i
                        .iter()
                        .zip(f_x_b.iter())
                        .map(|(coeff, v)| (*coeff - v) * (w_y_i - point.1).invert().unwrap())
                        .collect::<Vec<E::Fr>>()
                })
                .collect::<Vec<E::Fr>>();

            best_multiexp(
                &q_1_x_y,
                prover_param
                    .borrow()
                    .powers_of_g_lagrange_over_both_roots
                    .as_ref(),
            )
            .to_affine()
        };
        end_timer!(timer2);
        let proof = BiKZGProof::<E> {
            pi0: pi_0,
            pi1: pi_1,
        };

        end_timer!(timer);
        (proof, u)
    }

    fn verify(
        verifier_param: &Self::VerifierParam,
        commitment: &Self::Commitment,
        point: &Self::Point,
        value: &Self::Evaluation,
        proof: &Self::Proof,
    ) -> bool
    where
        E: MultiMillerLoop,
    {
        let pi0_a_pi1_b_g1_cmu = best_multiexp(
            &[point.0, point.1, E::Fr::ONE, -*value],
            &[proof.pi0, proof.pi1, commitment.com, verifier_param.g],
        );
        let pi0_a_pi1_b_g1_cmu = (-pi0_a_pi1_b_g1_cmu).to_affine();
        let res = E::multi_miller_loop(&[
            (&proof.pi0, &verifier_param.tau_0_h.into()),
            (&proof.pi1, &verifier_param.tau_1_h.into()),
            (&pi0_a_pi1_b_g1_cmu, &verifier_param.h.into()),
        ]);

        res.final_exponentiation().is_identity().into()
    }

    // TODO: implement multi-opening and batch verification
}
