use std::marker::PhantomData;

use arith::{ExtensionField, FFTField};
use polynomials::MultiLinearPoly;
use serdes::ExpSerde;

use crate::{
    fri_commit, fri_open, fri_verify, FRICommitment, FRIOpening, FRIScratchPad,
    PolynomialCommitmentScheme,
};

pub struct FRIVanillaPCS<F, ExtF, const RATE_LOG2: usize>
where
    F: FFTField + ExpSerde,
    ExtF: ExtensionField + From<F> + ExpSerde + FFTField,
{
    _marker_f: PhantomData<F>,
    _marker_ext_f: PhantomData<ExtF>,
}

impl<F, ExtF, const RATE_LOG2: usize> PolynomialCommitmentScheme<ExtF>
    for FRIVanillaPCS<F, ExtF, RATE_LOG2>
where
    F: FFTField + ExpSerde,
    ExtF: ExtensionField + From<F> + ExpSerde + FFTField,
{
    const NAME: &'static str = "FRIBaseFieldPCS";

    type Params = usize;
    type Poly = MultiLinearPoly<F>;
    type EvalPoint = Vec<ExtF>;
    type ScratchPad = FRIScratchPad<F>;

    type SRS = ();
    type Commitment = FRICommitment;
    type Opening = FRIOpening<ExtF>;

    fn gen_srs_for_testing(params: &Self::Params, _rng: impl rand::RngCore) -> (Self::SRS, usize) {
        const MIN_LEAVES: usize = 2;

        let min_vars = MIN_LEAVES
            .div_ceil(F::FIELD_SIZE)
            .next_power_of_two()
            .ilog2() as usize;

        ((), std::cmp::max(*params, min_vars))
    }

    fn init_scratch_pad(_params: &Self::Params) -> Self::ScratchPad {
        FRIScratchPad::default()
    }

    fn commit(
        _params: &Self::Params,
        _pk: &<Self::SRS as gkr_engine::StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        fri_commit(poly, RATE_LOG2, scratch_pad)
    }

    fn open(
        _params: &Self::Params,
        _pk: &<Self::SRS as gkr_engine::StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        scratch_pad: &Self::ScratchPad,
        transcript: &mut impl gkr_engine::Transcript<ExtF>,
    ) -> (ExtF, Self::Opening) {
        fri_open(poly, x, transcript, scratch_pad)
    }

    fn verify(
        _params: &Self::Params,
        _vk: &<Self::SRS as gkr_engine::StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: ExtF,
        opening: &Self::Opening,
        transcript: &mut impl gkr_engine::Transcript<ExtF>,
    ) -> bool {
        fri_verify::<F, ExtF>(commitment, x, v, opening, transcript)
    }
}
