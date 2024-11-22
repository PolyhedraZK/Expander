use arith::{Field, FieldSerde};
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use polynomials::MultiLinearPoly;
use rand::RngCore;
use std::fmt::Debug;
use transcript::Transcript;

pub trait StructuredReferenceString {
    type PKey: Clone + Debug + FieldSerde + Send;
    type VKey: Clone + Debug + FieldSerde + Send;

    /// Convert the SRS into proving and verifying keys.
    /// Comsuming self by default.
    fn into_keys(self) -> (Self::PKey, Self::VKey);
}

/// Standard Polynomial commitment scheme (PCS) trait.
pub trait PolynomialCommitmentScheme<F: Field> {
    const NAME: &'static str;

    type Params: Clone + Debug + Default;
    type Poly: Clone + Debug + Default;
    type EvalPoint: Clone + Debug + Default;
    type ScratchPad: Clone + Debug + Default;

    type SRS: Clone + Debug + Default + FieldSerde + StructuredReferenceString;
    type Commitment: Clone + Debug + Default + FieldSerde;
    type Opening: Clone + Debug + Default + FieldSerde;

    /// Generate a random structured reference string (SRS) for testing purposes.
    /// Use self as the first argument to save some potential intermediate state.
    fn gen_srs_for_testing(params: &Self::Params, rng: impl RngCore) -> Self::SRS;

    /// Initialize the scratch pad.
    fn init_scratch_pad(params: &Self::Params) -> Self::ScratchPad;

    /// Commit to a polynomial.
    fn commit(
        params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment;

    /// Open the polynomial at a point.
    fn open(
        params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        scratch_pad: &mut Self::ScratchPad,
    ) -> (F, Self::Opening);

    /// Verify the opening of a polynomial at a point.
    fn verify(
        params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: F,
        opening: &Self::Opening,
    ) -> bool;
}

pub struct ExpanderGKRChallenge<C: GKRFieldConfig> {
    pub x: Vec<C::ChallengeField>,
    pub x_simd: Vec<C::ChallengeField>,
    pub x_mpi: Vec<C::ChallengeField>,
}

pub trait PCSForExpanderGKR<C: GKRFieldConfig, T: Transcript<C::ChallengeField>> {
    const NAME: &'static str;

    type Params: Clone + Debug + Default + Send;
    type ScratchPad: Clone + Debug + Default + Send;

    type SRS: Clone + Debug + Default + FieldSerde + StructuredReferenceString;
    type Commitment: Clone + Debug + Default + FieldSerde;
    type Opening: Clone + Debug + Default + FieldSerde;

    /// Generate a random structured reference string (SRS) for testing purposes.
    /// Each process should return the SAME GLOBAL SRS.
    fn gen_srs_for_testing(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        rng: impl RngCore,
    ) -> Self::SRS;

    fn gen_params(n_input_vars: usize) -> Self::Params;

    /// Initialize the scratch pad.
    /// Each process returns its own scratch pad.
    fn init_scratch_pad(params: &Self::Params, mpi_config: &MPIConfig) -> Self::ScratchPad;

    /// Commit to a polynomial. Root process returns the commitment, other processes can return
    /// arbitrary value.
    fn commit(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &MultiLinearPoly<C::SimdCircuitField>,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment;

    /// Open the polynomial at a point.
    /// Root process returns the opening, other processes can return arbitrary value.
    ///
    /// Note(ZF): In GKR, We'll add the opening proof to the transcript after
    /// calling this function.
    /// However, if the open function itself is a multi-round interactive argument,
    /// `transcript.append_field_element` is likely to be used within the function.
    ///
    /// By default, `transcript.append_field_element` will add the field element to the proof,
    /// which means the field element is added twice.
    ///
    /// A temporary solution is to add a `transcript.lock_proof()` at the beginning of the open
    /// function and a `transcript.unlock_proof()` at the end of the open function.
    ///
    /// In this case, the `lock/unlock` function must be added at the beginning and end of the
    /// verify function as well.
    fn open(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &MultiLinearPoly<C::SimdCircuitField>,
        x: &ExpanderGKRChallenge<C>,
        transcript: &mut T, // add transcript here to allow interactive arguments
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Opening;

    /// Verify the opening of a polynomial at a point.
    /// This should only be called on the root process.
    #[allow(clippy::too_many_arguments)]
    fn verify(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &ExpanderGKRChallenge<C>,
        v: C::ChallengeField,
        transcript: &mut T, // add transcript here to allow interactive arguments
        opening: &Self::Opening,
    ) -> bool;
}
