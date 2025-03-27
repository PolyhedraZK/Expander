use arith::{ExtensionField, Field};
use gkr_engine::{StructuredReferenceString, Transcript};
use rand::RngCore;
use serdes::ExpSerde;
use std::fmt::Debug;

/// Standard Polynomial commitment scheme (PCS) trait.
pub trait PolynomialCommitmentScheme<F: ExtensionField, T: Transcript<F>> {
    const NAME: &'static str;

    type Params: Clone + Debug + Default;
    type Poly: Clone + Debug + Default;
    type EvalPoint: Clone + Debug + Default;
    type ScratchPad: Clone + Debug + Default + ExpSerde;

    type SRS: Clone + Debug + Default + ExpSerde + StructuredReferenceString;
    type Commitment: Clone + Debug + Default + ExpSerde;
    type Opening: Clone + Debug + Default + ExpSerde;

    /// Minimum number of variables supported in this PCS implementation,
    /// that such constraint exists for PCSs like Orion,
    /// but for Raw and Hyrax, polys of any size works.
    const MINIMUM_NUM_VARS: usize = 0;

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
        scratch_pad: &Self::ScratchPad,
        transcript: &mut T,
    ) -> (F, Self::Opening);

    /// Verify the opening of a polynomial at a point.
    fn verify(
        params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: F,
        opening: &Self::Opening,
        transcript: &mut T,
    ) -> bool;
}

pub(crate) trait TensorCodeIOPPCS {
    const LEAVES_IN_RANGE_OPENING: usize = 2;

    fn codeword_len(&self) -> usize;

    fn minimum_hamming_weight(&self) -> f64;

    fn evals_shape<F: Field>(num_vars: usize) -> (usize, usize) {
        let elems_for_smallest_tree = tree::leaf_adic::<F>() * Self::LEAVES_IN_RANGE_OPENING;

        let row_num: usize = elems_for_smallest_tree;
        let msg_size: usize = (1 << num_vars) / row_num;

        (row_num, msg_size)
    }

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
