use std::ops::{Mul, Neg};
use std::{borrow::Borrow, marker::PhantomData};

use arith::{
    lagrange_coefficients, parallelize, powers_of_field_elements, univariate_quotient,
    UnivariatePolynomial,
};
use ark_std::log2;
use halo2curves::ff::PrimeField;
use halo2curves::group::prime::PrimeCurveAffine;
use halo2curves::group::Curve;
use halo2curves::group::Group;
use halo2curves::msm::best_multiexp;
use halo2curves::pairing::MillerLoopResult;
use halo2curves::CurveAffine;
use halo2curves::{ff::Field, pairing::MultiMillerLoop};
use rand::RngCore;

use crate::{
    PolynomialCommitmentScheme, UniKZGCommitment, UniKZGOepning, UniKZGSRS, UniVerifierParam,
};

/// Commit to the bi-variate polynomial in its coefficient form.
/// Note that it is in general more efficient to use the lagrange form.
pub struct CoeffFormUniKZG<E: MultiMillerLoop> {
    _phantom: PhantomData<E>,
}

impl<E: MultiMillerLoop> PolynomialCommitmentScheme for CoeffFormUniKZG<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    type SRS = UniKZGSRS<E>;
    type ProverParam = UniKZGSRS<E>;
    type VerifierParam = UniVerifierParam<E>;
    type Polynomial = UnivariatePolynomial<E::Fr>;
    type Commitment = UniKZGCommitment<E>;
    type Proof = UniKZGOepning<E>;
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
        alpha: &Self::Point,
    ) -> (Self::Proof, Self::Evaluation) {
        // open a polynomial f(x) at point alpha
        // we compute h(x) = (f(x) - f(alpha))/(x - alpha)
        // and commit to h(x), and return h(alpha) as the evaluation

        let f_alpha = polynomial.evaluate(alpha);

        // h(x) = (f(x) - f(alpha))/(x - alpha)
        let hx = univariate_quotient(&polynomial.coefficients, alpha);

        let open = UniKZGOepning {
            opening: best_multiexp(hx.as_slice(), prover_param.borrow().g.as_slice()).to_affine(),
        };

        (open, f_alpha)
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
        // we want to verify that f(x) - f(alpha) == h(x) * (x - alpha)
        // which is to verify
        // e( g_1^(f(x))*g_1^(-f(alpha)), g_2 ) == e( g_1^(h(x)), g_2^x * g_2^(-alpha) ) )
        // which is
        // e( commit * g_1^-eval, g_2 ) == e( proof, pp.s_g2 * g_2^(-alpha) )

        let g1_eval = E::G1Affine::generator().mul(value.neg());
        let g2_alpha = E::G2Affine::generator().mul(point.neg());

        let pairing_result = E::multi_miller_loop(&[
            (
                &(-(commitment.commitment.to_curve() + g1_eval)).into(),
                &E::G2Affine::generator().into(),
            ),
            (
                &proof.opening,
                &((verifier_param.s_g2.to_curve() + g2_alpha)
                    .to_affine()
                    .into()),
            ),
        ]);

        pairing_result.final_exponentiation().is_identity().into()
    }

    // todo: implement batch opening and batch verification

}
