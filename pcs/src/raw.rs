use std::marker::PhantomData;

use crate::{EmptyType, PCS};
use arith::{Field, FieldSerde};
use polynomials::MultiLinearPoly;
use rand::RngCore;
use transcript::Transcript;

#[derive(Clone, Debug)]
pub struct RawMLParams {
    pub n_vars: usize,
}

// Raw commitment for multi-linear polynomials
pub struct RawML<F: Field + FieldSerde, T: Transcript<F>> {
    pub _phantom_f: PhantomData<F>,
    pub _phantom_t: PhantomData<T>,
}

impl<F: Field + FieldSerde, T: Transcript<F>> PCS for RawML<F, T> {
    type Params = RawMLParams;

    type Poly = MultiLinearPoly<F>;

    type EvalPoint = Vec<F>;

    type Eval = F;

    type SRS = EmptyType;

    type PKey = EmptyType;

    type VKey = EmptyType;

    type Commitment = Vec<F>;

    type Opening = EmptyType;

    type FiatShamirTranscript = T;

    fn gen_srs_for_testing(&mut self, _rng: impl RngCore, _params: &Self::Params) -> Self::SRS {
        Self::SRS::default()
    }

    fn commit(
        &mut self,
        params: &Self::Params,
        _proving_key: &Self::PKey,
        poly: &Self::Poly,
    ) -> Self::Commitment {
        assert!(1 << params.n_vars == poly.coeffs.len());
        poly.coeffs.clone()
    }

    fn open(
        &mut self,
        params: &Self::Params,
        _proving_key: &Self::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        _transcript: &mut Self::FiatShamirTranscript,
    ) -> (Self::Eval, Self::Opening) {
        assert!(1 << params.n_vars == poly.coeffs.len());
        (poly.evaluate_jolt(x), Self::Opening::default())
    }

    fn verify(
        params: &Self::Params,
        _verifying_key: &Self::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: Self::Eval,
        _opening: &Self::Opening,
        _transcript: &mut Self::FiatShamirTranscript,
    ) -> bool {
        assert!(1 << params.n_vars == commitment.len());
        let ml_poly = MultiLinearPoly::<F> {
            coeffs: commitment.clone(),
        };
        ml_poly.evaluate_jolt(x) == v
    }
}
