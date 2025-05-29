use std::marker::PhantomData;

use arith::ExtensionField;
use arith::Field;
use ark_std::log2;
use gkr_engine::{StructuredReferenceString, Transcript};
use halo2curves::group::Curve;
use halo2curves::group::Group;
use halo2curves::{
    ff::PrimeField,
    pairing::{Engine, MultiMillerLoop},
    CurveAffine,
};
use polynomials::{EqPolynomial, MultiLinearPoly, MultilinearExtension};
use serdes::ExpSerde;
use sumcheck::SumCheck;
use sumcheck::SumOfProductsPoly;

use crate::{
    traits::{BatchOpening, BatchOpeningPCS},
    *,
};
use kzg::hyper_kzg::*;

pub struct HyperKZGPCS<E>
where
    E: Engine,
    E::Fr: ExtensionField,
{
    _marker_e: PhantomData<E>,
}

impl<E> HyperKZGPCS<E>
where
    E: Engine,
    E::Fr: ExtensionField,
{
    pub const MINIMUM_SUPPORTED_NUM_VARS: usize = 2;
}

impl<E> PolynomialCommitmentScheme<E::Fr> for HyperKZGPCS<E>
where
    E: Engine + MultiMillerLoop,
    E::Fr: ExtensionField + PrimeField,
    E::G1Affine: ExpSerde + Default + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::G2Affine: ExpSerde + Default + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    const NAME: &'static str = "HyperKZGPCS";

    type Params = usize;
    type Poly = MultiLinearPoly<E::Fr>;
    type EvalPoint = Vec<E::Fr>;
    type ScratchPad = ();

    type SRS = CoefFormUniKZGSRS<E>;
    type Commitment = KZGCommitment<E>;
    type Opening = HyperKZGOpening<E>;

    fn init_scratch_pad(_params: &Self::Params) -> Self::ScratchPad {}

    fn gen_srs_for_testing(params: &Self::Params, rng: impl rand::RngCore) -> (Self::SRS, usize) {
        let local_num_vars = if *params == 0 { 1 } else { *params };

        let length = 1 << local_num_vars;
        let srs = generate_coef_form_uni_kzg_srs_for_testing(length, rng);
        (srs, local_num_vars)
    }

    fn commit(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        KZGCommitment(coeff_form_uni_kzg_commit(proving_key, &poly.coeffs))
    }

    fn open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        _scratch_pad: &Self::ScratchPad,
        transcript: &mut impl Transcript,
    ) -> (E::Fr, Self::Opening) {
        coeff_form_uni_hyperkzg_open(proving_key, &poly.coeffs, x, transcript)
    }

    fn verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: E::Fr,
        opening: &Self::Opening,
        transcript: &mut impl Transcript,
    ) -> bool {
        coeff_form_uni_hyperkzg_verify(verifying_key, commitment.0, x, v, opening, transcript)
    }
}

impl<E> BatchOpeningPCS<E::Fr> for HyperKZGPCS<E>
where
    E: Engine + MultiMillerLoop,
    E::Fr: ExtensionField + PrimeField,
    E::G1Affine: ExpSerde + Default + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::G2Affine: ExpSerde + Default + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    fn single_point_batch_open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        polys: &[Self::Poly],
        x: &Self::EvalPoint,
        _scratch_pad: &Self::ScratchPad,
        transcript: &mut impl Transcript,
    ) -> (Vec<E::Fr>, Self::Opening) {
        kzg_batch_open(proving_key, polys, x, transcript)
    }

    fn single_point_batch_verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitments: &[Self::Commitment],
        x: &Self::EvalPoint,
        evals: &[E::Fr],
        opening: &Self::Opening,
        transcript: &mut impl Transcript,
    ) -> bool {
        let commitment_unwrapped = commitments.iter().map(|c| c.0).collect::<Vec<_>>();

        kzg_batch_verify(
            verifying_key,
            &commitment_unwrapped,
            x,
            evals,
            opening,
            transcript,
        )
    }

    /// Open a set of polynomials at a multiple points.
    /// Requires the length of the polys to be the same as points.
    /// Steps:
    /// 1. get challenge point t from transcript
    /// 2. build eq(t,i) for i in [0..k]
    /// 3. build \tilde g_i(b) = eq(t, i) * f_i(b)
    /// 4. compute \tilde eq_i(b) = eq(b, point_i)
    /// 5. run sumcheck on \sum_i=1..k \tilde eq_i * \tilde g_i
    /// 6. build g'(X) = \sum_i=1..k \tilde eq_i(a2) * \tilde g_i(X) where (a2) is the sumcheck's
    ///    point
    /// 7. open g'(X) at point (a2)
    ///
    /// Returns:
    /// - the evaluations of the polynomials at their corresponding points
    /// - the batch opening proof containing the sumcheck proof and the opening of g'(X)
    fn multiple_points_batch_open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        polys: &[Self::Poly],
        points: &[Self::EvalPoint],
        _scratch_pad: &Self::ScratchPad,
        transcript: &mut impl Transcript,
    ) -> (Vec<E::Fr>, BatchOpening<E::Fr, Self>) {
        let num_vars = polys[0].num_vars();
        let k = polys.len();
        let ell = log2(k) as usize;

        // generate evals for each polynomial at its corresponding point
        let evals: Vec<E::Fr> = polys
            .iter()
            .zip(points.iter())
            .map(|(poly, point)| poly.evaluate_jolt(point))
            .collect();

        // challenge point t
        let t = transcript.generate_field_elements::<E::Fr>(ell);

        // eq(t, i) for i in [0..k]
        let eq_t_i = EqPolynomial::build_eq_x_r(&t);

        // \tilde g_i(b) = eq(t, i) * f_i(b)
        let mut tilde_gs = vec![];
        for (index, f_i) in polys.iter().enumerate() {
            let mut tilde_g_eval = vec![E::Fr::zero(); 1 << num_vars];
            for (j, &f_i_eval) in f_i.coeffs.iter().enumerate() {
                tilde_g_eval[j] = f_i_eval * eq_t_i[index];
            }
            tilde_gs.push(MultiLinearPoly {
                coeffs: tilde_g_eval,
            });
        }

        // built the virtual polynomial for SumCheck
        let tilde_eqs: Vec<MultiLinearPoly<E::Fr>> = points
            .iter()
            .map(|point| {
                let eq_b_zi = EqPolynomial::build_eq_x_r(point);
                MultiLinearPoly { coeffs: eq_b_zi }
            })
            .collect();

        let mut sumcheck_poly = SumOfProductsPoly::new();
        for (tilde_g, tilde_eq) in tilde_gs.iter().zip(tilde_eqs.into_iter()) {
            sumcheck_poly.add_pair(tilde_g.clone(), tilde_eq);
        }

        let proof = SumCheck::<E::Fr>::prove(&sumcheck_poly, transcript);

        let a2 = &proof.point[..num_vars];
        let mut a2_rev = a2.to_vec();
        a2_rev.reverse();

        // build g'(X) = \sum_i=1..k \tilde eq_i(a2) * \tilde g_i(X) where (a2) is the
        // sumcheck's point \tilde eq_i(a2) = eq(a2, point_i)
        let mut g_prime_evals = vec![E::Fr::zero(); 1 << num_vars];

        for (tilde_g, point) in tilde_gs.iter().zip(points.iter()) {
            let eq_i_a2 = EqPolynomial::eq_vec(a2_rev.as_ref(), point);
            for (j, &tilde_g_eval) in tilde_g.coeffs.iter().enumerate() {
                g_prime_evals[j] += tilde_g_eval * eq_i_a2;
            }
        }
        let g_prime = MultiLinearPoly {
            coeffs: g_prime_evals,
        };

        let (_g_prime_eval, g_prime_proof) =
            coeff_form_uni_hyperkzg_open(proving_key, &g_prime.coeffs, a2_rev.as_ref(), transcript);
        (
            evals,
            BatchOpening {
                sum_check_proof: proof,
                g_prime_proof,
            },
        )
    }

    /// Verify the opening of a set of polynomials at a single point.
    /// Steps:
    /// 1. get challenge point t from transcript
    /// 2. build g' commitment
    /// 3. ensure \sum_i eq(a2, point_i) * eq(t, <i>) * f_i_evals matches the sum via SumCheck
    ///    verification
    /// 4. verify commitment
    fn multiple_points_batch_verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitments: &[Self::Commitment],
        points: &[Self::EvalPoint],
        values: &[E::Fr],
        opening: &BatchOpening<E::Fr, Self>,
        transcript: &mut impl Transcript,
    ) -> bool {
        let k = commitments.len();
        let ell = log2(k) as usize;
        let num_var = opening.sum_check_proof.point.len();

        // sum check point (a2)
        let a2 = &opening.sum_check_proof.point[..num_var];
        let mut a2_rev = a2.to_vec();
        a2_rev.reverse();

        // challenge point t
        let t = transcript.generate_field_elements::<E::Fr>(ell);

        let eq_t_i = EqPolynomial::build_eq_x_r(&t);

        // build g' commitment

        // todo: use MSM
        // let mut scalars = vec![];
        // let mut bases = vec![];

        let mut g_prime_commit_elems = E::G1::identity();
        for (i, point) in points.iter().enumerate() {
            let eq_i_a2 = EqPolynomial::eq_vec(a2_rev.as_ref(), point);
            let scalar = eq_i_a2 * eq_t_i[i];

            g_prime_commit_elems += commitments[i].0 * scalar;
        }

        let g_prime_commit = g_prime_commit_elems.to_affine();

        // ensure \sum_i eq(t, <i>) * f_i_evals matches the sum via SumCheck
        let mut sum = E::Fr::zero();
        for (i, &e) in eq_t_i.iter().enumerate().take(k) {
            sum += e * values[i];
        }

        let subclaim =
            SumCheck::<E::Fr>::verify(sum, &opening.sum_check_proof, num_var, transcript);

        let tilde_g_eval = subclaim.expected_evaluation;

        // verify commitment
        coeff_form_uni_hyperkzg_verify(
            verifying_key,
            g_prime_commit,
            a2_rev.as_ref(),
            tilde_g_eval,
            &opening.g_prime_proof,
            transcript,
        )
        // hyrax_verify(
        //     verifying_key,
        //     &g_prime_commit,
        //     a2_rev.as_ref(),
        //     tilde_g_eval,
        //     &opening.g_prime_proof,
        // )
    }
}
