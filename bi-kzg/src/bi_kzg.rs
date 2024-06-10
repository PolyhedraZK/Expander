use std::{borrow::Borrow, marker::PhantomData, slice::ChunkBy};

use halo2curves::ff::Field;
use halo2curves::ff::PrimeField;
use halo2curves::group::prime::PrimeCurveAffine;
use halo2curves::group::Curve;
use halo2curves::group::Group;

use halo2curves::pairing::Engine;
use rand::Rng;
use rand::RngCore;

use crate::util::lagrange_coefficients;
use crate::util::parallelize;
use crate::{
    pcs::PolynomialCommitmentScheme,
    util::{powers_of_field_elements, tensor_product_parallel},
    BiKZGCommitment, BiKZGProof, BiKZGProverParam, BiKZGSRS, BiKZGVerifierParam,
};

pub struct BiKZG<E: Engine> {
    _engine: PhantomData<E>,
}

impl<E: Engine> PolynomialCommitmentScheme for BiKZG<E> {
    type SRS = BiKZGSRS<E>;
    type ProverParam = BiKZGProverParam<E>;
    type VerifierParam = BiKZGVerifierParam<E>;
    type Polynomial = Vec<E::Fr>;
    type Commitment = BiKZGCommitment<E>;
    type Proof = BiKZGProof<E>;
    type Evaluation = E::Fr;
    type Point = E::Fr;
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
        // let tau_0 = E::Fr::from(5);
        // let tau_1 = E::Fr::from(7);

        let g1 = E::G1Affine::generator();

        // roots of unity for supported_n and supported_m
        let (omega_0, omega_1) = {
            let omega = E::Fr::ROOT_OF_UNITY;
            let omega_0 = omega.pow_vartime(&[(1 << E::Fr::S) / supported_n as u64]);
            let omega_1 = omega.pow_vartime(&[(1 << E::Fr::S) / supported_m as u64]);

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
        let scalars = {
            let powers_of_omega_0 = powers_of_field_elements(omega_0, supported_n);
            let powers_of_tau_0 = powers_of_field_elements(tau_0, supported_n);
            let lagrange_tau_0 = lagrange_coefficients(&powers_of_omega_0, &powers_of_tau_0);
            let powers_of_omega_1 = powers_of_field_elements(omega_1, supported_m);
            let powers_of_tau_1 = powers_of_field_elements(tau_1, supported_m);
            let lagrange_tau_1 = lagrange_coefficients(&powers_of_omega_1, &powers_of_tau_1);
            tensor_product_parallel(&lagrange_tau_0, &lagrange_tau_1)
        };

        println!("start to compute the affine bases");
        let affine_bases = {
            let mut proj_bases = vec![E::G1::identity(); supported_n * supported_m];
            parallelize(&mut proj_bases, |g, start| {
                for (idx, g) in g.iter_mut().enumerate() {
                    let offset = start + idx;
                    *g = g1 * scalars[offset];
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
            powers_of_g: affine_bases,
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
        unimplemented!()
    }

    fn open(
        prover_param: impl Borrow<Self::ProverParam>,
        polynomial: &Self::Polynomial,
        point: &Self::Point,
    ) -> (Self::Proof, Self::Evaluation) {
        unimplemented!()
    }

    fn verify(
        verifier_param: &Self::VerifierParam,
        commitment: &Self::Commitment,
        point: &Self::Point,
        value: &Self::Evaluation,
        proof: &Self::Proof,
    ) -> bool {
        unimplemented!()
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
