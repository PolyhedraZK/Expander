use std::marker::PhantomData;

use arith::ExtensionField;
use halo2curves::{ff::PrimeField, CurveAffine};
use polynomials::MultiLinearPoly;
use serdes::ExpSerde;
use transcript::Transcript;

use crate::{
    hyrax::hyrax_impl::{hyrax_commit, hyrax_open, hyrax_setup, hyrax_verify},
    HyraxCommitment, HyraxOpening, PedersenParams, PolynomialCommitmentScheme,
};

pub struct HyraxPCS<C, T>
where
    C: CurveAffine + ExpSerde,
    T: Transcript<C::Scalar>,
    C::Scalar: ExtensionField,
    C::ScalarExt: ExtensionField,
{
    _phantom_c: PhantomData<C>,
    _phantom_t: PhantomData<T>,
}

impl<C, T> PolynomialCommitmentScheme<C::Scalar, T> for HyraxPCS<C, T>
where
    C: CurveAffine + ExpSerde,
    T: Transcript<C::Scalar>,
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

    fn gen_srs_for_testing(params: &Self::Params, rng: impl rand::RngCore) -> Self::SRS {
        hyrax_setup(*params, rng)
    }

    fn commit(
        _params: &Self::Params,
        proving_key: &<Self::SRS as crate::StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        hyrax_commit(proving_key, poly)
    }

    fn open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as crate::StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        _scratch_pad: &Self::ScratchPad,
        _transcript: &mut T,
    ) -> (C::Scalar, Self::Opening) {
        hyrax_open(proving_key, poly, x)
    }

    fn verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as crate::StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: C::Scalar,
        opening: &Self::Opening,
        _transcript: &mut T,
    ) -> bool {
        hyrax_verify(verifying_key, commitment, x, v, opening)
    }
}
