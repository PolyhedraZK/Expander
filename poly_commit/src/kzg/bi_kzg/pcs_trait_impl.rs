use std::marker::PhantomData;

use arith::ExtensionField;
use ark_ec::pairing::Pairing;
use ark_std::rand::RngCore;
use gkr_engine::{StructuredReferenceString, Transcript};
use polynomials::{MultiLinearPoly, MultilinearExtension};
use serdes::ExpSerde;

use crate::{
    coeff_form_uni_hyperkzg_open, coeff_form_uni_hyperkzg_verify, coeff_form_uni_kzg_commit,
    HyperUniKZGOpening, PolynomialCommitmentScheme,
};

use super::{
    generate_coef_form_bi_kzg_local_srs_for_testing, BiKZGCommitment, CoefFormBiKZGLocalSRS,
    HyperBiKZGOpening,
};

pub struct HyperBiKZGPCS<E>
where
    E: Pairing,
    E::ScalarField: ExtensionField,
{
    _marker_e: PhantomData<E>,
}

impl<E> HyperBiKZGPCS<E>
where
    E: Pairing,
    E::ScalarField: ExtensionField,
{
    pub const MINIMUM_SUPPORTED_NUM_VARS: usize = 2;
}

impl<E> PolynomialCommitmentScheme<E::ScalarField> for HyperBiKZGPCS<E>
where
    E: Pairing,
    E::ScalarField: ExtensionField,
    E::G1Affine: ExpSerde + Default,
    E::G2Affine: ExpSerde + Default,
{
    const NAME: &'static str = "HyperBiKZGPCS";

    type Params = usize;
    type Poly = MultiLinearPoly<E::ScalarField>;
    type EvalPoint = Vec<E::ScalarField>;
    type ScratchPad = ();

    type SRS = CoefFormBiKZGLocalSRS<E>;
    type Commitment = BiKZGCommitment<E>;
    type Opening = HyperBiKZGOpening<E>;

    fn init_scratch_pad(_params: &Self::Params) -> Self::ScratchPad {}

    fn gen_srs_for_testing(params: &Self::Params, rng: impl RngCore) -> (Self::SRS, usize) {
        let local_num_vars = if *params == 0 { 1 } else { *params };

        let length = 1 << local_num_vars;
        let srs = generate_coef_form_bi_kzg_local_srs_for_testing(length, 1, 0, rng);
        (srs, local_num_vars)
    }

    fn commit(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        let local_commitment =
            coeff_form_uni_kzg_commit(&proving_key.tau_x_srs, poly.hypercube_basis_ref());

        BiKZGCommitment(local_commitment)
    }

    fn open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        _scratch_pad: &Self::ScratchPad,
        transcript: &mut impl Transcript,
    ) -> (E::ScalarField, Self::Opening) {
        let (eval, hyperkzg_opening) = coeff_form_uni_hyperkzg_open(
            &proving_key.tau_x_srs,
            poly.hypercube_basis_ref(),
            x,
            transcript,
        );

        let hyper_bikzg_opening: HyperBiKZGOpening<E> = hyperkzg_opening.into();
        (eval, hyper_bikzg_opening)
    }

    fn verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: E::ScalarField,
        opening: &Self::Opening,
        transcript: &mut impl Transcript,
    ) -> bool {
        let hyper_bikzg_opening = opening.clone();
        let hyper_kzg_opening: HyperUniKZGOpening<E> = hyper_bikzg_opening.into();

        coeff_form_uni_hyperkzg_verify(
            &verifying_key.into(),
            commitment.0,
            x,
            v,
            &hyper_kzg_opening,
            transcript,
        )
    }
}
