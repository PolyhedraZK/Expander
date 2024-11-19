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

pub(crate) trait TensorCodeIOPPCS {
    fn codeword_len(&self) -> usize;

    fn hamming_weight(&self) -> f64;

    fn evals_shape<F: Field>(num_vars: usize) -> (usize, usize) {
        let elems_for_smallest_tree = tree::leaf_adic::<F>() * 2;

        let row_num: usize = elems_for_smallest_tree;
        let msg_size: usize = (1 << num_vars) / row_num;

        (row_num, msg_size)
    }

    fn query_complexity(&self, soundness_bits: usize) -> usize {
        // NOTE: use Ligero (AHIV22) or Avg-case dist to a code (BKS18)
        // version of avg case dist in unique decoding technique.
        let avg_case_dist = self.hamming_weight() / 3f64;
        let sec_bits = -(1f64 - avg_case_dist).log2();

        (soundness_bits as f64 / sec_bits).ceil() as usize
    }

    fn proximity_repetitions<F: Field>(&self, soundness_bits: usize) -> usize {
        // NOTE: use Ligero (AHIV22) or Avg-case dist to a code (BKS18)
        // version of avg case dist in unique decoding technique.
        // Here is the probability union bound
        let single_run_soundness_bits = F::FIELD_SIZE - self.codeword_len().ilog2() as usize;

        (soundness_bits as f64 / single_run_soundness_bits as f64).ceil() as usize
    }
}
