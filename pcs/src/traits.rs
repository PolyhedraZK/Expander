use arith::{Field, FieldSerde};
use rand::RngCore;
use std::{borrow::Borrow, fmt::Debug};
use transcript::Transcript;

pub trait PolynomialCommitmentScheme {
    type PublicParams: Clone + Debug;
    type Poly: Clone + Debug;
    type EvalPoint: Clone + Debug;
    type Eval: Field + FieldSerde;

    type SRS: Clone + Debug + FieldSerde;
    type ProverKey: Clone + Debug + From<Self::SRS> + FieldSerde;
    // TODO: verifier key should be small, can be derived from a reference-like obj
    type VerifierKey: Clone + Debug + From<Self::SRS> + FieldSerde;

    type CommitmentWithData: Clone + Debug;
    type Commitment: Clone + Debug + FieldSerde + From<Self::CommitmentWithData>;
    type OpeningProof: Clone + Debug + FieldSerde;

    type FiatShamirTranscript: Transcript<Self::Eval>;

    fn gen_srs_for_testing(rng: impl RngCore, params: &Self::PublicParams) -> Self::SRS;

    fn commit(
        params: &Self::PublicParams,
        proving_key: impl Borrow<Self::ProverKey>,
        poly: &Self::Poly,
    ) -> Self::CommitmentWithData;

    fn open(
        params: &Self::PublicParams,
        proving_key: impl Borrow<Self::ProverKey>,
        poly: &Self::Poly,
        opening_point: &Self::EvalPoint,
        transcript: &mut Self::FiatShamirTranscript,
    ) -> (Self::Eval, Self::OpeningProof);

    fn verify(
        params: &Self::PublicParams,
        verifying_key: &Self::VerifierKey,
        commitment: &Self::Commitment,
        opening_point: &Self::EvalPoint,
        evaluation: Self::Eval,
        opening: &Self::OpeningProof,
        transcript: &mut Self::FiatShamirTranscript,
    ) -> bool;
}
