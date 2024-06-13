use std::ops::{Add, AddAssign};
use std::process::Output;
use std::{borrow::Borrow, marker::PhantomData, slice::ChunkBy};

use halo2curves::ff::Field;
use halo2curves::ff::PrimeField;
use halo2curves::group::prime::PrimeCurveAffine;
use halo2curves::group::Curve;
use halo2curves::group::Group;

// use halo2curves::msm::best_multiexp;
use halo2curves::pairing::{MillerLoopResult, MultiMillerLoop};
use halo2curves::pasta::pallas::Scalar;
use halo2curves::CurveAffine;
use rand::Rng;
use rand::RngCore;

use crate::msm::best_multiexp;
use crate::poly::{lagrange_coefficients, lagrange_coefficients_a};
use crate::structs::BivariatePolynomial;
use crate::util::parallelize;
use crate::{
    pcs::PolynomialCommitmentScheme,
    util::{powers_of_field_elements, tensor_product_parallel},
    BiKZGCommitment, BiKZGProof, BiKZGSRS, BiKZGVerifierParam,
};

pub struct BiKZG<E: MultiMillerLoop> {
    _phantom: PhantomData<E>,
}

impl<E: MultiMillerLoop> PolynomialCommitmentScheme for BiKZG<E>
where
    E::G1Affine: Add<Output = E::G1>,
{
    type SRS = BiKZGSRS<E>;
    type ProverParam = BiKZGSRS<E>;
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

        // let tau_0 = E::Fr::random(&mut rng);
        // let tau_1 = E::Fr::random(&mut rng);
        let tau_0 = E::Fr::from(5);
        let tau_1 = E::Fr::from(7);

        let g1 = E::G1Affine::generator();

        // roots of unity for supported_n and supported_m
        let (omega_0, omega_1) = {
            let omega = E::Fr::ROOT_OF_UNITY;
            let omega_0 = omega.pow_vartime(&[(1 << E::Fr::S) / supported_n as u64]);
            let omega_1 = omega.pow_vartime(&[(1 << E::Fr::S) / supported_m as u64]);

            println!("omega 0: {:?}", omega_0);
            println!("omega 1: {:?}", omega_1);

            println!("n: {}", supported_n);
            println!("m: {}", supported_m);

            assert!(
                omega_0.pow_vartime(&[supported_n as u64]) == E::Fr::ONE,
                "omega_0 is not root of unity for supported_n"
            );
            assert!(
                omega_1.pow_vartime(&[supported_m as u64]) == E::Fr::ONE,
                "omega_1 is not root of unity for supported_m"
            );
            (omega_0, omega_1)
        };

        println!("start to compute the scalars");
        // computes the vector of L_i^N(tau_0) * L_j^M(tau_1) for i in 0..supported_n and j in 0..supported_m
        let (scalars, lagrange_scalars) = {
            let powers_of_omega_0 = powers_of_field_elements(&omega_0, supported_n);
            let powers_of_tau_0 = powers_of_field_elements(&tau_0, supported_n);
            let lagrange_tau_0 = lagrange_coefficients_a(&powers_of_omega_0, &tau_0);
            let powers_of_omega_1 = powers_of_field_elements(&omega_1, supported_m);
            let powers_of_tau_1 = powers_of_field_elements(&tau_1, supported_m);
            let lagrange_tau_1 = lagrange_coefficients_a(&powers_of_omega_1, &tau_1);
            let scalars = tensor_product_parallel(&powers_of_tau_0, &powers_of_tau_1);
            let lagrange_scalars = tensor_product_parallel(&lagrange_tau_0, &lagrange_tau_1);


            println!("lagrange_tau_0: {:?}", lagrange_tau_0);
            println!("lagrange_tau_1: {:?}", lagrange_tau_1);
            (scalars, lagrange_scalars)
        };

        println!("lagrange scalars: {:?} ", lagrange_scalars);

        println!("start to compute the affine bases");
        let coeff_bases = {
            let mut proj_bases = vec![E::G1::identity(); supported_n * supported_m];
            parallelize(&mut proj_bases, |g, start| {
                for (idx, g) in g.iter_mut().enumerate() {
                    let offset = start + idx;
                    *g = g1 * scalars[offset];
                }
            });

            let mut g_bases = vec![E::G1Affine::identity(); supported_n * supported_m];
            parallelize(&mut g_bases, |g, starts| {
                E::G1::batch_normalize(&proj_bases[starts..(starts + g.len())], g);
            });
            drop(proj_bases);
            g_bases
        };

        println!("start to compute the lagrange bases");
        println!("lagrange scalars: {:?} ", lagrange_scalars);
        let lagrange_bases = {
            let mut proj_bases = vec![E::G1::identity(); supported_n * supported_m];
            parallelize(&mut proj_bases, |g, start| {
                for (idx, g) in g.iter_mut().enumerate() {
                    let offset = start + idx;
                    *g = g1 * lagrange_scalars[offset];
                }
            });

            let mut g_lagrange = vec![E::G1Affine::identity(); supported_n * supported_m];
            parallelize(&mut g_lagrange, |g_lagrange, starts| {
                E::G1::batch_normalize(
                    &proj_bases[starts..(starts + g_lagrange.len())],
                    g_lagrange,
                );
            });
            drop(proj_bases);
            g_lagrange
        };

        BiKZGSRS {
            tau_0,
            tau_1,
            powers_of_g: coeff_bases,
            powers_of_g_lagrange: lagrange_bases,
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
        let com = best_multiexp(
            &poly.coefficients,
            prover_param.borrow().powers_of_g.as_slice(),
        )
        .into();

        assert_eq!(
            com,
            (prover_param.borrow().powers_of_g[0]
                * poly.evaluate(&prover_param.borrow().tau_0, &prover_param.borrow().tau_1))
            .into(),
            "commitment is not equal to evaluation"
        );

        let lag_coeff = poly.lagrange_coeffs();
        let com_lag = best_multiexp(
            &lag_coeff,
            prover_param.borrow().powers_of_g_lagrange.as_slice(),
        )
        .into();
        assert_eq!(
            com, com_lag,
            "commitment is not equal to lagrange commitment"
        );
        println!("finished commit");
        Self::Commitment { com }
    }

    fn open(
        prover_param: impl Borrow<Self::ProverParam>,
        polynomial: &Self::Polynomial,
        point: &Self::Point,
    ) -> (Self::Proof, Self::Evaluation) {
        // fixme
        let tau_0 = prover_param.borrow().tau_0;
        let tau_1 = prover_param.borrow().tau_1;
        let a = point.0;
        let b = point.1;
        let c = polynomial.evaluate(&tau_0, &tau_1);

        let u = polynomial.evaluate(&a, &b);
        let u_prime = polynomial.evaluate(&tau_0, &b);

        let f_tau0_b = polynomial.evaluate(&tau_0, &b);
        let f_a_tau1 = polynomial.evaluate(&a, &tau_1);

        println!("here {:?} {:?} {:?} {:?}", tau_0, tau_1, a, b);
        let q_0 = (f_tau0_b - u) * ((tau_0 - a).invert().unwrap());
        let q_1 = (c - u_prime) * ((tau_1 - b).invert().unwrap());

        println!("here2");
        let proof = BiKZGProof {
            pi0: (prover_param.borrow().powers_of_g[0] * q_0).into(),
            pi1: (prover_param.borrow().powers_of_g[0] * q_1).into(),
        };

        let t0 = q_0 * (tau_0 - a);
        let t1 = q_1 * (tau_1 - b);
        let right = c - u;

        assert_eq!(t0 + t1, right, "t0 + t1 != right");

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
            &[
                proof.pi0,
                proof.pi1,
                commitment.com.into(),
                verifier_param.g.into(),
            ],
        );
        let pi0_a_pi1_b_g1_cmu = (-pi0_a_pi1_b_g1_cmu).to_affine();
        let res = E::multi_miller_loop(&[
            (&proof.pi0, &verifier_param.tau_0_h.into()),
            (&proof.pi1, &verifier_param.tau_1_h.into()),
            (&pi0_a_pi1_b_g1_cmu, &verifier_param.h.into()),
        ]);
        let res = res.final_exponentiation().is_identity().into();

        println!("res: {:?}", res);
        res
    }

    fn multi_open(
        _prover_param: impl Borrow<Self::ProverParam>,
        _polynomials: &[Self::Polynomial],
        _points: &[Self::Point],
        _evals: &[Self::Evaluation],
        // _transcript: &mut IOPTranscript<E::ScalarField>,
    ) -> Self::BatchProof {
        unimplemented!()
    }

    fn batch_verify(
        _verifier_param: &Self::VerifierParam,
        _commitments: &[Self::Commitment],
        _points: &[Self::Point],
        _batch_proof: &Self::BatchProof,
    ) -> bool {
        unimplemented!()
    }
}
