use arith::Field;
use polynomials::MultilinearExtension;
use rand::RngCore;
use serdes::ExpSerde;
use std::{fmt::Debug, str::FromStr};

use crate::{ExpErrors, ExpanderSingleVarChallenge, FieldEngine, MPIEngine, Transcript};

pub trait StructuredReferenceString {
    type PKey: Clone + Debug + ExpSerde + Send + Sync + 'static;
    type VKey: Clone + Debug + ExpSerde + Send + Sync + 'static;

    /// Convert the SRS into proving and verifying keys.
    /// Comsuming self by default.
    fn into_keys(self) -> (Self::PKey, Self::VKey);
}

pub trait ExpanderPCS<F: FieldEngine> {
    const NAME: &'static str;

    const PCS_TYPE: PolynomialCommitmentType;

    type Params: Clone + Debug + Default + Send + Sync + 'static;
    type ScratchPad: Clone + Debug + Default + Send + ExpSerde + Sync;

    type SRS: Clone + Debug + Default + ExpSerde + StructuredReferenceString + Send + Sync;
    type Commitment: Clone + Debug + Default + ExpSerde;
    type Opening: Clone + Debug + Default + ExpSerde;
    
    type PolyField: Field;

    /// Minimum number of variables supported in this PCS implementation,
    /// that such constraint exists for PCSs like Orion,
    /// but for Raw and Hyrax, polys of any size works.
    const MINIMUM_NUM_VARS: usize = 0;

    /// Generate a random structured reference string (SRS) for testing purposes.
    /// Each process should return the SAME GLOBAL SRS.
    fn gen_srs_for_testing(
        params: &Self::Params,
        mpi_engine: &impl MPIEngine,
        rng: impl RngCore,
    ) -> Self::SRS;

    /// n_input_vars is with respect to the multilinear poly on each machine in MPI,
    /// also ignore the number of variables stacked in the SIMD field.
    fn gen_params(n_input_vars: usize) -> Self::Params;

    /// Initialize the scratch pad.
    /// Each process returns its own scratch pad.
    fn init_scratch_pad(params: &Self::Params, mpi_engine: &impl MPIEngine) -> Self::ScratchPad;

    /// Commit to a polynomial. Root process returns the commitment, other processes can return
    /// arbitrary value.
    fn commit(
        params: &Self::Params,
        mpi_engine: &impl MPIEngine,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl MultilinearExtension<Self::PolyField>,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Option<Self::Commitment>;

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
    ///
    /// NOTE(HS): We introduce MPI for the sake of parallelism, s.t., we can accelerate
    /// the opening algorithm.  In such case, only the PCS opening at the root matters,
    /// while opening from the subordinate parties are not used, at a scope of whole GKR
    /// argument system.
    fn open(
        params: &Self::Params,
        mpi_engine: &impl MPIEngine,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl MultilinearExtension<Self::PolyField>,
        x: &ExpanderSingleVarChallenge<F::ChallengeField>,
        transcript: &mut impl Transcript,
        scratch_pad: &Self::ScratchPad,
    ) -> Option<Self::Opening>;

    /// Verify the opening of a polynomial at a point.
    /// This should only be called on the root process.
    ///
    /// NOTE(HS): Again, corresponding to the comments in opening, the PCS opening reaching
    /// this verify algorithm should be the one at the MPI root, rather than the ones from
    /// any other subordinate MPI parties.
    fn verify(
        params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &ExpanderSingleVarChallenge<F::ChallengeField>,
        v: F::ChallengeField,
        transcript: &mut impl Transcript,
        opening: &Self::Opening,
    ) -> bool;
}

impl StructuredReferenceString for () {
    type PKey = ();
    type VKey = ();

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        ((), ())
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum PolynomialCommitmentType {
    #[default]
    Raw,
    KZG,
    Hyrax,
    Orion,
    FRI,
}

impl FromStr for PolynomialCommitmentType {
    type Err = ExpErrors;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Raw" => Ok(PolynomialCommitmentType::Raw),
            "KZG" => Ok(PolynomialCommitmentType::KZG),
            "Hyrax" => Ok(PolynomialCommitmentType::Hyrax),
            "Orion" => Ok(PolynomialCommitmentType::Orion),
            "FRI" => Ok(PolynomialCommitmentType::FRI),
            _ => Err(ExpErrors::PCSTypeError(s.to_string())),
        }
    }
}
