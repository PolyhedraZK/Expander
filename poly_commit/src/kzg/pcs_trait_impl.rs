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

use crate::*;
use kzg::hyper_kzg::*;

pub struct HyperKZGPCS<E, T>
where
    E: Engine,
    E::Fr: ExtensionField,
    T: Transcript<E::Fr>,
{
    _marker_e: PhantomData<E>,
    _marker_t: PhantomData<T>,
}

impl<E, T> PolynomialCommitmentScheme<E::Fr, T> for HyperKZGPCS<E, T>
where
    E: Engine + MultiMillerLoop,
    E::Fr: ExtensionField + PrimeField,
    E::G1Affine: ExpSerde + Default + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::G2Affine: ExpSerde + Default + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
    T: Transcript<E::Fr>,
{
    const NAME: &'static str = "HyperKZGPCS";

    type Params = usize;
    type Poly = MultiLinearPoly<E::Fr>;
    type EvalPoint = Vec<E::Fr>;
    type ScratchPad = ();

    type SRS = CoefFormUniKZGSRS<E>;
    type Commitment = KZGCommitment<E>;
    type Opening = HyperKZGOpening<E>;

    const MINIMUM_NUM_VARS: usize = 1;

    fn init_scratch_pad(_params: &Self::Params) -> Self::ScratchPad {}

    fn gen_srs_for_testing(params: &Self::Params, rng: impl rand::RngCore) -> Self::SRS {
        let length = 1 << params;
        generate_coef_form_uni_kzg_srs_for_testing(length, rng)
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
        transcript: &mut T,
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
        transcript: &mut T,
    ) -> bool {
        coeff_form_uni_hyperkzg_verify(
            verifying_key.clone(),
            commitment.0,
            x,
            v,
            opening,
            transcript,
        )
    }
}
