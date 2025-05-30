use std::marker::PhantomData;

use arith::ExtensionField;
use gkr_engine::{StructuredReferenceString, Transcript};
use halo2curves::{
    ff::PrimeField,
    pairing::{Engine, MultiMillerLoop},
    CurveAffine,
};
use polynomials::MultiLinearPoly;
use serdes::ExpSerde;

use crate::batching::prover_merge_points;
use crate::batching::verifier_merge_points;
use crate::{
    traits::{BatchOpening, BatchOpeningPCS},
    *,
};

use super::batch::{kzg_single_point_batch_open, kzg_single_point_batch_verify};

pub struct HyperUniKZGPCS<E>
where
    E: Engine,
    E::Fr: ExtensionField,
{
    _marker_e: PhantomData<E>,
}

impl<E> PolynomialCommitmentScheme<E::Fr> for HyperUniKZGPCS<E>
where
    E: Engine + MultiMillerLoop,
    E::Fr: ExtensionField + PrimeField,
    E::G1Affine: ExpSerde + Default + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::G2Affine: ExpSerde + Default + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    const NAME: &'static str = "HyperUniKZGPCS";

    type Params = usize;
    type Poly = MultiLinearPoly<E::Fr>;
    type EvalPoint = Vec<E::Fr>;
    type ScratchPad = ();

    type SRS = CoefFormUniKZGSRS<E>;
    type Commitment = UniKZGCommitment<E>;
    type Opening = HyperUniKZGOpening<E>;

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
        UniKZGCommitment(coeff_form_uni_kzg_commit(proving_key, &poly.coeffs))
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

impl<E> BatchOpeningPCS<E::Fr> for HyperUniKZGPCS<E>
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
        kzg_single_point_batch_open(proving_key, polys, x, transcript)
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

        kzg_single_point_batch_verify(
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
        // generate evals for each polynomial at its corresponding point
        let evals: Vec<E::Fr> = polys
            .iter()
            .zip(points.iter())
            .map(|(poly, point)| poly.evaluate_jolt(point))
            .collect();

        let (new_point, g_prime, proof) =
            prover_merge_points::<E::G1Affine>(polys, points, transcript);

        let (_g_prime_eval, g_prime_proof) =
            coeff_form_uni_hyperkzg_open(proving_key, &g_prime.coeffs, &new_point, transcript);
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
        batch_opening: &BatchOpening<E::Fr, Self>,
        transcript: &mut impl Transcript,
    ) -> bool {
        // sum check point (a2)
        let a2 = batch_opening.sum_check_proof.export_point_to_expander();

        let commitments = commitments.iter().map(|c| vec![c.0]).collect::<Vec<_>>();

        let (tilde_g_eval, g_prime_commit) = verifier_merge_points::<E::G1Affine>(
            &commitments,
            points,
            values,
            &batch_opening.sum_check_proof,
            transcript,
        );

        // verify commitment
        coeff_form_uni_hyperkzg_verify(
            verifying_key,
            g_prime_commit[0],
            a2.as_ref(),
            tilde_g_eval,
            &batch_opening.g_prime_proof,
            transcript,
        )
    }
}
