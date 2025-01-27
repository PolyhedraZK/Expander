use std::{borrow::Borrow, marker::PhantomData};

use ark_std::{end_timer, start_timer};
use halo2curves::ff::Field;
use halo2curves::ff::PrimeField;
use halo2curves::group::prime::PrimeCurveAffine;
use halo2curves::group::Curve;
use halo2curves::group::Group;
use halo2curves::msm::best_multiexp;
use halo2curves::pairing::{MillerLoopResult, MultiMillerLoop};
use halo2curves::CurveAffine;
use itertools::Itertools;
use rand::RngCore;

use crate::poly::{
    lagrange_coefficients, univariate_quotient, BivariateLagrangePolynomial, BivariatePolynomial,
};
use crate::util::parallelize;
use crate::{
    pcs::PolynomialCommitmentScheme,
    util::{powers_of_field_elements, tensor_product_parallel},
    BiKZGCommitment, BiKZGProof, BiKZGVerifierParam, CoefFormBiKZGSRS,
};

/// Commit to the bi-variate polynomial in its coefficient form.
/// Note that it is in general more efficient to use the lagrange form.
pub struct CoeffFormBiKZG<E: MultiMillerLoop> {
    _phantom: PhantomData<E>,
}

impl<E: MultiMillerLoop> PolynomialCommitmentScheme for CoeffFormBiKZG<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    type SRS = CoefFormBiKZGSRS<E>;
    type ProverParam = CoefFormBiKZGSRS<E>;
    type VerifierParam = BiKZGVerifierParam<E>;
    type Polynomial = BivariatePolynomial<E::Fr>;
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
        let (omega_0, omega_1) = {
            let omega = E::Fr::ROOT_OF_UNITY;
            let omega_0 = omega.pow_vartime([(1 << E::Fr::S) / supported_n as u64]);
            let omega_1 = omega.pow_vartime([(1 << E::Fr::S) / supported_m as u64]);

            assert!(
                omega_0.pow_vartime([supported_n as u64]) == E::Fr::ONE,
                "omega_0 is not root of unity for supported_n"
            );
            assert!(
                omega_1.pow_vartime([supported_m as u64]) == E::Fr::ONE,
                "omega_1 is not root of unity for supported_m"
            );
            (omega_0, omega_1)
        };

        // computes the vector of L_i^N(tau_0) * L_j^M(tau_1)
        // for i in 0..supported_n and j in 0..supported_m
        let (scalars, lagrange_scalars) = {
            let powers_of_omega_0 = powers_of_field_elements(&omega_0, supported_n);
            let powers_of_tau_0 = powers_of_field_elements(&tau_0, supported_n);
            let lagrange_tau_0 = lagrange_coefficients(&powers_of_omega_0, &tau_0);
            let powers_of_omega_1 = powers_of_field_elements(&omega_1, supported_m);
            let powers_of_tau_1 = powers_of_field_elements(&tau_1, supported_m);
            let lagrange_tau_1 = lagrange_coefficients(&powers_of_omega_1, &tau_1);
            let scalars = tensor_product_parallel(&powers_of_tau_0, &powers_of_tau_1);
            let lagrange_scalars = tensor_product_parallel(&lagrange_tau_0, &lagrange_tau_1);

            (scalars, lagrange_scalars)
        };

        let g1_prog = g1.to_curve();
        let coeff_bases = {
            let mut proj_bases = vec![E::G1::identity(); supported_n * supported_m];
            parallelize(&mut proj_bases, |g, start| {
                for (idx, g) in g.iter_mut().enumerate() {
                    let offset = start + idx;
                    *g = g1_prog * scalars[offset];
                }
            });

            let mut g_bases = vec![E::G1Affine::identity(); supported_n * supported_m];
            parallelize(&mut g_bases, |g, starts| {
                E::G1::batch_normalize(&proj_bases[starts..(starts + g.len())], g);
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
            parallelize(&mut affine_bases, |affine_bases, starts| {
                E::G1::batch_normalize(
                    &proj_bases[starts..(starts + affine_bases.len())],
                    affine_bases,
                );
            });
            drop(proj_bases);
            affine_bases
        };

        CoefFormBiKZGSRS {
            powers_of_g: coeff_bases,
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
            "Committing to polynomial of degree {} {}",
            poly.degree_0, poly.degree_1
        ));
        let timer2 =
            start_timer!(|| format!("Computing the msm for size {}", poly.coefficients.len()));
        let com = best_multiexp(
            &poly.coefficients,
            prover_param.borrow().powers_of_g.as_slice(),
        );
        end_timer!(timer2);
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
        // u = f(a, b)
        let u = polynomial.evaluate(&a, &b);

        let timer2 = start_timer!(|| "Computing the proof pi0");
        let (pi_0, f_x_b) = {
            // t = f(x, b) - f(a, b)
            let f_x_b = polynomial.evaluate_at_y(&point.1);
            let mut t = f_x_b.clone();
            t[0] -= u;
            // q_0(x, b) = t(x) / (x - a)
            let q_0_x_b = univariate_quotient(&t, &point.0);

            let timer2 = start_timer!(|| format!("Computing the msm for size {}", q_0_x_b.len()));
            let pi_0 = best_multiexp(
                &q_0_x_b,
                prover_param.borrow().powers_of_g[..polynomial.degree_0].as_ref(),
            )
            .to_affine();
            end_timer!(timer2);
            (pi_0, f_x_b)
        };
        end_timer!(timer2);

        let timer2 = start_timer!(|| "Computing the proof pi1");
        let pi_1 = {
            // t = f(x, y) - f(x, b)
            let mut t = polynomial.clone();
            t.coefficients
                .iter_mut()
                .take(polynomial.degree_0)
                .zip_eq(f_x_b.iter())
                .for_each(|(c, f)| *c -= f);
            let t_lag = t.interpolate();

            // divisor = (y - b)
            let divisor_lag = BivariateLagrangePolynomial::from_y_monomial(
                &point.1,
                polynomial.degree_0,
                polynomial.degree_1,
            );
            let y_minus_a_inv_lag = divisor_lag
                .coefficients
                .iter()
                .map(|o| {
                    if o.is_zero_vartime() {
                        panic!("not invertible")
                    } else {
                        o.invert().unwrap()
                    }
                })
                .collect::<Vec<_>>();

            // q_1(x, y) = t(x, y) * (y - b)^{-1}
            let q_1_x_y = t_lag
                .iter()
                .zip_eq(y_minus_a_inv_lag.iter())
                .map(|(c, y)| (*c) * *y)
                .collect::<Vec<_>>();

            let timer2 = start_timer!(|| format!("Computing the msm for size {}", q_1_x_y.len()));
            let res = best_multiexp(
                &q_1_x_y,
                prover_param
                    .borrow()
                    .powers_of_g_lagrange_over_both_roots
                    .as_ref(),
            )
            .to_affine();
            end_timer!(timer2);
            res
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
        let timers = start_timer!(|| "Verifying the proof");
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
        let res = res.final_exponentiation().is_identity().into();
        end_timer!(timers);
        res
    }

    // TODO: implement multi-opening and batch verification
}
