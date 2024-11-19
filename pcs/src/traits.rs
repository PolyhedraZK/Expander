use arith::{Field, FieldSerde};
use gkr_field_config::GKRFieldConfig;
use polynomials::MultiLinearPoly;
use rand::RngCore;
use std::fmt::Debug;

pub trait SRS {
    type PKey: Clone + Debug + FieldSerde;
    type VKey: Clone + Debug + FieldSerde;

    /// Convert the SRS into proving and verifying keys.
    /// Comsuming self by default.
    fn into_keys(self) -> (Self::PKey, Self::VKey);
}

/// Standard Polynomial commitment scheme (PCS) trait.
pub trait PCS<F: Field + FieldSerde> {
    const NAME: &'static str;

    type Params: Clone + Debug + Default;
    type Poly: Clone + Debug + Default;
    type EvalPoint: Clone + Debug + Default;
    type ScratchPad: Clone + Debug + Default;

    type SRS: Clone + Debug + Default + FieldSerde + SRS;
    type Commitment: Clone + Debug + Default + FieldSerde;
    type Opening: Clone + Debug + Default + FieldSerde;

    /// Generate a random structured reference string (SRS) for testing purposes.
    /// Use self as the first argument to save some potential intermediate state.
    fn gen_srs_for_testing(params: &Self::Params, rng: impl RngCore) -> Self::SRS;

    /// Commit to a polynomial.
    fn commit(
        proving_key: &<Self::SRS as SRS>::PKey,
        poly: &Self::Poly,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment;

    /// Open the polynomial at a point.
    fn open(
        proving_key: &<Self::SRS as SRS>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        scratch_pad: &mut Self::ScratchPad,
    ) -> (F, Self::Opening);

    /// Verify the opening of a polynomial at a point.
    fn verify(
        verifying_key: &<Self::SRS as SRS>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: F,
        opening: &Self::Opening,
    ) -> bool;
}

pub trait PCSForGKR<C: GKRFieldConfig>:
    PCS<
    C::ChallengeField,
    Poly = MultiLinearPoly<C::SimdCircuitField>,
    EvalPoint = (
        Vec<C::ChallengeField>, // x
        Vec<C::ChallengeField>, // x_simd
        Vec<C::ChallengeField>, // x_mpi
    ),
>
{
}
