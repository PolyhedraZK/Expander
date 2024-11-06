use crate::{EmptyType, PCS};
use arith::{Field, FieldSerde};
use polynomials::MultiLinearPoly;
use rand::RngCore;

#[derive(Clone, Debug)]
pub struct RawMLParams {
    pub n_vars: usize,
}

// Raw commitment for multi-linear polynomials
pub struct RawML {}

impl<F: Field + FieldSerde> PCS<F> for RawML {
    type Params = RawMLParams;

    type Poly = MultiLinearPoly<F>;

    type EvalPoint = Vec<F>;

    type SRS = EmptyType;

    type PKey = EmptyType;

    type VKey = EmptyType;

    type Commitment = Vec<F>;

    type Opening = EmptyType;

    fn gen_srs_for_testing(&self, _rng: impl RngCore, _params: &Self::Params) -> Self::SRS {
        Self::SRS::default()
    }

    fn commit(
        &self,
        params: &Self::Params,
        _proving_key: &Self::PKey,
        poly: &Self::Poly,
    ) -> Self::Commitment {
        assert!(1 << params.n_vars == poly.coeffs.len());
        poly.coeffs.clone()
    }

    fn open(
        &self,
        params: &Self::Params,
        _proving_key: &Self::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
    ) -> (F, Self::Opening) {
        assert!(1 << params.n_vars == poly.coeffs.len());
        (poly.evaluate_jolt(x), Self::Opening::default())
    }

    fn verify(
        &self,
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
