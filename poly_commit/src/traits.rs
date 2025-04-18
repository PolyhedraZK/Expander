use arith::{ExtensionField, Field};
use gkr_engine::{StructuredReferenceString, Transcript};
use rand::RngCore;
use serdes::ExpSerde;
use std::fmt::Debug;

/// Standard Polynomial commitment scheme (PCS) trait.
pub trait PolynomialCommitmentScheme<F: ExtensionField> {
    const NAME: &'static str;

    type Params: Clone + Debug + Default;
    type Poly: Clone + Debug + Default;
    type EvalPoint: Clone + Debug + Default;
    type ScratchPad: Clone + Debug + Default + ExpSerde;

    type SRS: Clone + Debug + Default + ExpSerde + StructuredReferenceString;
    type Commitment: Clone + Debug + Default + ExpSerde;
    type Opening: Clone + Debug + Default + ExpSerde;

    /// Generate a random structured reference string (SRS) for testing purposes.
    /// Use self as the first argument to save some potential intermediate state.
    ///
    /// Additionally, this method returns a calibrated number of variables for
    /// the polynomial, that the PCS might need to accept a length extended
    /// version of polynomial as input.
    fn gen_srs_for_testing(params: &Self::Params, rng: impl RngCore) -> (Self::SRS, usize);

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
        scratch_pad: &Self::ScratchPad,
        transcript: &mut impl Transcript<F>,
    ) -> (F, Self::Opening);

    /// Verify the opening of a polynomial at a point.
    fn verify(
        params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: F,
        opening: &Self::Opening,
        transcript: &mut impl Transcript<F>,
    ) -> bool;
}

pub(crate) trait TensorCodeIOPPCS {
    fn message_len(&self) -> usize;

    fn codeword_len(&self) -> usize;

    fn minimum_hamming_weight(&self) -> f64;

    fn num_leaves_per_mt_query(&self) -> usize;

    fn query_complexity(&self, soundness_bits: usize) -> usize {
        // NOTE: use Ligero (AHIV22) appendix C argument.
        let avg_case_dist = self.minimum_hamming_weight() / 2f64;
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
