use std::ops::Mul;
use std::{borrow::Borrow, marker::PhantomData};

use arith::{lagrange_coefficients, parallelize, powers_of_field_elements, UnivariatePolynomial};
use ark_std::log2;
use halo2curves::ff::PrimeField;
use halo2curves::group::prime::PrimeCurveAffine;
use halo2curves::group::Curve;
use halo2curves::group::Group;
use halo2curves::msm::best_multiexp;
use halo2curves::CurveAffine;
use halo2curves::{ff::Field, pairing::MultiMillerLoop};
use rand::RngCore;

use crate::{PolynomialCommitmentScheme, UniKZGCommitment, UniKZGSRS};

/// Commit to the bi-variate polynomial in its coefficient form.
/// Note that it is in general more efficient to use the lagrange form.
pub struct CoeffFormBiKZG<E: MultiMillerLoop> {
    _phantom: PhantomData<E>,
}

impl<E: MultiMillerLoop> PolynomialCommitmentScheme for CoeffFormBiKZG<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    type SRS = UniKZGSRS<E>;
    type ProverParam = UniKZGSRS<E>;
    type VerifierParam = ();
    type Polynomial = UnivariatePolynomial<E::Fr>;
    type Commitment = UniKZGCommitment<E>;
    type Proof = ();
    type Evaluation = E::Fr;
    type Point = E::Fr;
    type BatchProof = ();

    fn gen_srs_for_testing(
        mut rng: impl RngCore,
        supported_n: usize,
        _supported_m: usize,
    ) -> Self::SRS {
        let log_degree = log2(supported_n);

        // root of unity
        let mut omega = E::Fr::ROOT_OF_UNITY;
        omega = omega.pow_vartime(&[1 << (E::Fr::S - log_degree)]);

        // toxic waste
        let tau = E::Fr::random(&mut rng);

        let g1 = E::G1Affine::generator();
        let g1_prog = g1.to_curve();
        let powers_of_omega = powers_of_field_elements(&omega, 1 << log_degree);
        let powers_of_tau = powers_of_field_elements(&tau, 1 << log_degree);

        let coeff_bases = {
            let mut proj_bases = vec![E::G1::identity(); 1 << log_degree];
            parallelize(&mut proj_bases, |g, start| {
                for (idx, g) in g.iter_mut().enumerate() {
                    let offset = start + idx;
                    *g = g1_prog * powers_of_tau[offset];
                }
            });

            let mut g_bases = vec![E::G1Affine::identity(); 1 << log_degree];
            parallelize(&mut g_bases, |g, starts| {
                E::G1::batch_normalize(&proj_bases[starts..(starts + g.len())], g);
            });
            drop(proj_bases);
            g_bases
        };

        let lagrange_tau = lagrange_coefficients(&powers_of_omega, &tau);

        let lagrange_bases = {
            let mut proj_bases = vec![E::G1::identity(); 1 << log_degree];
            parallelize(&mut proj_bases, |g, start| {
                for (idx, g) in g.iter_mut().enumerate() {
                    let offset = start + idx;
                    *g = g1 * lagrange_tau[offset];
                }
            });

            let mut affine_bases = vec![E::G1Affine::identity(); 1 << log_degree];
            parallelize(&mut affine_bases, |affine_bases, starts| {
                E::G1::batch_normalize(
                    &proj_bases[starts..(starts + affine_bases.len())],
                    affine_bases,
                );
            });
            drop(proj_bases);
            affine_bases
        };

        let g2 = E::G2Affine::generator();
        let s_g2 = g2.mul(tau).to_affine();

        UniKZGSRS {
            g: coeff_bases,
            g_lagrange: lagrange_bases,
            g2,
            s_g2,
        }
    }

    fn commit(
        prover_param: impl Borrow<Self::ProverParam>,
        poly: &Self::Polynomial,
    ) -> Self::Commitment {
        let commit = best_multiexp(
            poly.coefficients.as_slice(),
            prover_param.borrow().g.as_slice(),
        );

        UniKZGCommitment {
            commitment: commit.to_affine(),
        }
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
    ) -> bool
    where
        E: MultiMillerLoop,
    {
        true
    }
}
