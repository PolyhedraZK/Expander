use std::marker::PhantomData;

use arith::ExtensionField;
use arith::Field;
use ark_std::log2;
use gkr_engine::{StructuredReferenceString, Transcript};
use halo2curves::group::Curve;
use halo2curves::{ff::PrimeField, CurveAffine};
use polynomials::{EqPolynomial, MultiLinearPoly, MultilinearExtension};
use serdes::ExpSerde;
use sumcheck::SumCheck;

use crate::traits::BatchOpening;
use crate::{
    hyrax::hyrax_impl::{hyrax_commit, hyrax_open, hyrax_setup, hyrax_verify},
    traits::BatchOpeningPCS,
    HyraxCommitment, HyraxOpening, PedersenParams, PolynomialCommitmentScheme,
};

use super::hyrax_impl::{hyrax_batch_open, hyrax_batch_verify};

pub struct HyraxPCS<C>
where
    C: CurveAffine + ExpSerde,
    C::Scalar: ExtensionField,
    C::ScalarExt: ExtensionField,
{
    _phantom_c: PhantomData<C>,
}

impl<C> PolynomialCommitmentScheme<C::Scalar> for HyraxPCS<C>
where
    C: CurveAffine + ExpSerde,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
{
    const NAME: &'static str = "HyraxPCS";

    type Params = usize;
    type Poly = MultiLinearPoly<C::Scalar>;
    type EvalPoint = Vec<C::Scalar>;
    type ScratchPad = ();

    type SRS = PedersenParams<C>;
    type Commitment = HyraxCommitment<C>;
    type Opening = HyraxOpening<C>;

    fn init_scratch_pad(_params: &Self::Params) -> Self::ScratchPad {}

    fn gen_srs_for_testing(params: &Self::Params, rng: impl rand::RngCore) -> (Self::SRS, usize) {
        (hyrax_setup(*params, 0, rng), *params)
    }

    fn commit(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        hyrax_commit(proving_key, poly)
    }

    fn open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        _scratch_pad: &Self::ScratchPad,
        _transcript: &mut impl Transcript,
    ) -> (C::Scalar, Self::Opening) {
        hyrax_open(proving_key, poly, x)
    }

    fn verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: C::Scalar,
        opening: &Self::Opening,
        _transcript: &mut impl Transcript,
    ) -> bool {
        hyrax_verify(verifying_key, commitment, x, v, opening)
    }
}

impl<C> BatchOpeningPCS<C::Scalar> for HyraxPCS<C>
where
    C: CurveAffine + ExpSerde,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
{
    fn single_point_batch_open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        mle_poly_list: &[Self::Poly],
        eval_point: &Self::EvalPoint,
        _scratch_pad: &Self::ScratchPad,
        transcript: &mut impl Transcript,
    ) -> (Vec<C::Scalar>, Self::Opening) {
        hyrax_batch_open(proving_key, mle_poly_list, eval_point, transcript)
    }

    fn single_point_batch_verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        comm_list: &[Self::Commitment],
        eval_point: &Self::EvalPoint,
        eval_list: &[C::Scalar],
        batch_proof: &Self::Opening,
        transcript: &mut impl Transcript,
    ) -> bool {
        hyrax_batch_verify(
            verifying_key,
            comm_list,
            eval_point,
            eval_list,
            batch_proof,
            transcript,
        )
    }

    /// Open a set of polynomials at a multiple points.
    /// Requires the length of the polys to be the same as points.
    fn multiple_points_batch_open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        polys: &[Self::Poly],
        points: &[Self::EvalPoint],
        _scratch_pad: &Self::ScratchPad,
        transcript: &mut impl Transcript,
    ) -> BatchOpening<C::Scalar, Self> {
        let num_vars = polys[0].num_vars();
        let k = polys.len();
        let ell = log2(k) as usize;

        // generate evals for each polynomial at its corresponding point
        let evals: Vec<C::Scalar> = polys
            .iter()
            .zip(points.iter())
            .map(|(poly, point)| poly.evaluate_jolt(point))
            .collect();

        // challenge point t
        let t = transcript.generate_field_elements::<C::Scalar>(ell);

        // eq(t, i) for i in [0..k]
        let eq_t_i_list = EqPolynomial::build_eq_x_r(&t);

        // \tilde g_i(b) = eq(t, i) * f_i(b)
        let mut tilde_gs = vec![];
        for (index, f_i) in polys.iter().enumerate() {
            let mut tilde_g_eval = vec![C::Scalar::zero(); 1 << num_vars];
            for (j, &f_i_eval) in f_i.coeffs.iter().enumerate() {
                tilde_g_eval[j] = f_i_eval * eq_t_i_list[index];
            }
            tilde_gs.push(MultiLinearPoly {
                coeffs: tilde_g_eval,
            });
        }

        // built the virtual polynomial for SumCheck
        let tilde_eqs: Vec<MultiLinearPoly<C::Scalar>> = points
            .iter()
            .map(|point| {
                let eq_b_zi = EqPolynomial::build_eq_x_r(point);
                MultiLinearPoly { coeffs: eq_b_zi }
            })
            .collect();

        let mut sumcheck_poly = vec![];
        for (tilde_g, tilde_eq) in tilde_gs.iter().zip(tilde_eqs.into_iter()) {
            sumcheck_poly.extend_from_slice([tilde_g.clone(), tilde_eq].as_slice());
        }
        println!("SumCheck poly: {:?}", sumcheck_poly.len());
        for poly in sumcheck_poly.iter() {
            println!("Poly: {:?}", poly.coeffs.len());
        }

        let proof = SumCheck::<C::Scalar>::prove(&sumcheck_poly, transcript);

        println!("SumCheck proof: {:?}", proof,);

        let a2 = &proof.point[..num_vars];

        // build g'(X) = \sum_i=1..k \tilde eq_i(a2) * \tilde g_i(X) where (a2) is the
        // sumcheck's point \tilde eq_i(a2) = eq(a2, point_i)
        let mut g_prime_evals = vec![C::Scalar::zero(); 1 << num_vars];
        for (tilde_g, point) in tilde_gs.iter().zip(points.iter()) {
            let eq_i_a2 = EqPolynomial::eq_vec(a2, point);
            for (j, &tilde_g_eval) in tilde_g.coeffs.iter().enumerate() {
                g_prime_evals[j] += tilde_g_eval * eq_i_a2;
            }
        }
        let g_prime = MultiLinearPoly {
            coeffs: g_prime_evals,
        };

        let (_g_prime_eval, g_prime_proof) =
            hyrax_open(proving_key, &g_prime, a2.to_vec().as_ref());

        BatchOpening {
            sum_check_proof: proof,
            f_i_eval_at_point_i: evals.to_vec(),
            g_prime_proof,
        }
    }

    /// Verify the opening of a set of polynomials at a single point.
    fn multiple_points_batch_verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitments: &[Self::Commitment],
        points: &[Self::EvalPoint],
        _values: &[C::Scalar],
        opening: &BatchOpening<C::Scalar, Self>,
        transcript: &mut impl Transcript,
    ) -> bool {
        let k = commitments.len();
        let ell = log2(k) as usize;
        let num_var = opening.sum_check_proof.point.len();

        // challenge point t
        let t = transcript.generate_field_elements::<C::Scalar>(ell);

        // sum check point (a2)
        let a2 = &opening.sum_check_proof.point[..num_var];

        // build g' commitment
        // todo: use MSM
        let eq_t_list = EqPolynomial::build_eq_x_r(&t);

        // let mut scalars = vec![];
        // let mut bases = vec![];

        let mut g_prime_commit_elems = vec![C::Curve::default(); commitments[0].0.len()];
        for (i, point) in points.iter().enumerate() {
            let eq_i_a2 = EqPolynomial::eq_vec(a2, point);
            // scalars.push(eq_i_a2 * eq_t_list[i]);
            // bases.push(commitments[i].0);
            let scalar = eq_i_a2 * eq_t_list[i];
            for (j, &base) in commitments[i].0.iter().enumerate() {
                g_prime_commit_elems[j] += (base * scalar);
            }
        }
        let mut g_prime_commit_affine = vec![C::default(); commitments[0].0.len()];
        C::Curve::batch_normalize(&g_prime_commit_elems, &mut g_prime_commit_affine);

        let g_prime_commit = HyraxCommitment(g_prime_commit_affine);

        // ensure \sum_i eq(t, <i>) * f_i_evals matches the sum via SumCheck
        let mut sum = C::Scalar::zero();
        for (i, &e) in eq_t_list.iter().enumerate().take(k) {
            sum += e * opening.f_i_eval_at_point_i[i];
        }

        let subclaim =
            SumCheck::<C::Scalar>::verify(sum, &opening.sum_check_proof, num_var, transcript);

        let tilde_g_eval = subclaim.expected_evaluation;

        // verify commitment
        hyrax_verify(
            verifying_key,
            &g_prime_commit,
            a2.to_vec().as_ref(),
            tilde_g_eval,
            &opening.g_prime_proof,
        )
    }
}
