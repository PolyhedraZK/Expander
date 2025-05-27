use std::marker::PhantomData;

use arith::ExtensionField;
use arith::Field;
use gkr_engine::{DeferredCheck, StructuredReferenceString, Transcript};
use halo2curves::group::Group;
use halo2curves::msm::multiexp_serial;
use halo2curves::{
    ff::PrimeField,
    group::Curve,
    pairing::{Engine, MultiMillerLoop},
    CurveAffine,
};
use polynomials::MultiLinearPoly;
use serdes::ExpSerde;

use crate::*;
use kzg::hyper_kzg::*;

use super::deferred_pairing::PairingAccumulator;

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
        let mut accumulator = PairingAccumulator::default();

        let partial_check = coeff_form_uni_hyperkzg_partial_verify(
            verifying_key.clone(),
            commitment.0,
            x,
            v,
            opening,
            transcript,
            &mut accumulator,
        );

        let pairing_check = accumulator.final_check();

        pairing_check && partial_check
    }

    fn batch_open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        polys: &[Self::Poly],
        x: &Self::EvalPoint,
        _scratch_pad: &Self::ScratchPad,
        transcript: &mut impl Transcript,
    ) -> (Vec<E::Fr>, Self::Opening) {
        let rlc_randomness = transcript.generate_field_element::<E::Fr>();
        let num_poly = polys.len();
        let rlcs = powers_series(&rlc_randomness, num_poly);
        let mut buf = vec![E::Fr::default(); polys[0].coeffs.len()];

        let merged_poly = polys
            .iter()
            .zip(rlcs.iter())
            .skip(1)
            .fold(polys[0].clone(), |acc, (poly, r)| acc + &(poly * r));

        let mut evals = polys
            .iter()
            .map(|p| MultiLinearPoly::evaluate_with_buffer(p.coeffs.as_ref(), x, &mut buf))
            .collect::<Vec<_>>();

        let (_batch_eval, open) =
            coeff_form_uni_hyperkzg_open(proving_key, &merged_poly.coeffs, x, transcript);

        {
            // sanity check: the merged evaluation should match the batch evaluation
            // this step is not necessary if the performance is critical
            let mut merged_eval = evals[0];
            for (eval, r) in evals.iter_mut().zip(rlcs.iter()).skip(1) {
                merged_eval += *eval * r;
            }
            assert_eq!(_batch_eval, merged_eval);
        }

        (evals, open)
    }

    fn batch_verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitments: &[Self::Commitment],
        x: &Self::EvalPoint,
        evals: &[E::Fr],
        opening: &Self::Opening,
        transcript: &mut impl Transcript,
    ) -> bool {
        let rlc_randomness = transcript.generate_field_element::<E::Fr>();
        let num_poly = commitments.len();
        let rlcs = powers_series(&rlc_randomness, num_poly);

        let commitments_local = commitments.iter().map(|c| c.0).collect::<Vec<_>>();

        // stay with single thread as the num_poly is usually small
        let mut merged_commitment = E::G1::identity();
        multiexp_serial(&rlcs, &commitments_local, &mut merged_commitment);

        let merged_eval = evals
            .iter()
            .zip(rlcs.iter())
            .fold(E::Fr::zero(), |acc, (e, r)| acc + (*e * r));

        Self::verify(
            _params,
            verifying_key,
            &KZGCommitment(merged_commitment.to_affine()),
            x,
            merged_eval,
            opening,
            transcript,
        )
    }
}
