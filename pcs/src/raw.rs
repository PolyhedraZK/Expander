/// Raw commitment for multi-linear polynomials

use crate::{PCSEmptyType, PCS, PCSForGKR};
use arith::{Field, FieldSerde};
use polynomials::MultiLinearPoly;
use rand::RngCore;

#[derive(Clone, Debug, Default)]
pub struct RawMLParams {
    pub n_vars: usize,
}

// Raw commitment for multi-linear polynomials
pub struct RawML {}

impl<F: Field + FieldSerde> PCS<F> for RawML {
    const NAME: &'static str = "RawML";

    type Params = RawMLParams;

    type Poly = MultiLinearPoly<F>;

    type EvalPoint = Vec<F>;

    type SRS = PCSEmptyType;
    type Commitment = Vec<F>;

    type Opening = PCSEmptyType;

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
    ) -> (F, Self::Opening) {
        assert!(1 << params.n_vars == poly.coeffs.len());
        (poly.evaluate_jolt(x), Self::Opening::default())
    }

    fn verify(
        params: &Self::Params,
        _verifying_key: &Self::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: F,
        _opening: &Self::Opening,
    ) -> bool {
        assert!(1 << params.n_vars == commitment.len());
        let ml_poly = MultiLinearPoly::<F> {
            coeffs: commitment.clone(),
        };
        ml_poly.evaluate_jolt(x) == v
    }
}
