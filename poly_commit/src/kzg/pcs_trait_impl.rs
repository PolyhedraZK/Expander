use std::marker::PhantomData;

use arith::{BN254Fr, ExtensionField};
use halo2curves::{bn256::Bn256, pairing::Engine};
use polynomials::MultiLinearPoly;
use transcript::Transcript;

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

impl<T: Transcript<BN254Fr>> PolynomialCommitmentScheme<BN254Fr, T> for HyperKZGPCS<Bn256, T> {
    const NAME: &'static str = "HyperKZGPCS";

    type Params = usize;
    type Poly = MultiLinearPoly<BN254Fr>;
    type EvalPoint = Vec<BN254Fr>;
    type ScratchPad = ();

    type SRS = CoefFormUniKZGSRS<Bn256>;
    type Commitment = KZGCommitment<Bn256>;
    type Opening = HyperKZGOpening<Bn256>;

    fn init_scratch_pad(#[allow(unused)] params: &Self::Params) -> Self::ScratchPad {}

    fn gen_srs_for_testing(params: &Self::Params, rng: impl rand::RngCore) -> Self::SRS {
        let length = 1 << params;
        generate_coef_form_uni_kzg_srs_for_testing(length, rng)
    }

    fn commit(
        #[allow(unused)] params: &Self::Params,
        proving_key: &<Self::SRS as crate::StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        #[allow(unused)] scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        KZGCommitment(coeff_form_uni_kzg_commit(proving_key, &poly.coeffs))
    }

    fn open(
        #[allow(unused)] params: &Self::Params,
        proving_key: &<Self::SRS as crate::StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        #[allow(unused)] scratch_pad: &Self::ScratchPad,
        transcript: &mut T,
    ) -> (BN254Fr, Self::Opening) {
        coeff_form_uni_hyperkzg_open(proving_key, &poly.coeffs, x, transcript)
    }

    fn verify(
        #[allow(unused)] params: &Self::Params,
        verifying_key: &<Self::SRS as crate::StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: BN254Fr,
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
