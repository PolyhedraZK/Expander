use arith::{Field, FieldSerde};
use rand::RngCore;
use std::fmt::Debug;
use transcript::Transcript;

pub trait PolynomialCommitmentScheme {
    type PublicParams: Clone + Debug;
    type Poly: Clone + Debug;
    type EvalPoint: Clone + Debug;
    type Eval: Field + FieldSerde;

    type SRS: Clone + Debug;
    type ProverKey: Clone + Debug + From<Self::SRS>;
    // TODO: verifier key should be small, can be derived from a reference-like obj
    type VerifierKey: Clone + Debug + From<Self::SRS>;

    type CommitmentWithData: Clone + Debug;
    type Commitment: Clone + Debug + From<Self::CommitmentWithData>;
    type OpeningProof: Clone + Debug;

    type FiatShamirTranscript: Transcript<Self::Eval>;

    fn gen_srs_for_testing(rng: impl RngCore, params: &Self::PublicParams) -> Self::SRS;

    fn commit(proving_key: &Self::ProverKey, poly: &Self::Poly) -> Self::CommitmentWithData;

    fn open(
        proving_key: &Self::ProverKey,
        poly: &Self::Poly,
        opening_point: &Self::EvalPoint,
        commitment_with_data: &Self::CommitmentWithData,
        transcript: &mut Self::FiatShamirTranscript,
    ) -> (Self::Eval, Self::OpeningProof);

    fn verify(
        verifying_key: &Self::VerifierKey,
        commitment: &Self::Commitment,
        opening_point: &Self::EvalPoint,
        evaluation: Self::Eval,
        opening_proof: &Self::OpeningProof,
        transcript: &mut Self::FiatShamirTranscript,
    ) -> bool;
}
