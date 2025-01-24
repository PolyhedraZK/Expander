use std::marker::PhantomData;

use arith::{ExtensionField, Field, SimdField};
use polynomials::MultiLinearPoly;
use transcript::Transcript;

use crate::{
    orion::*, traits::TensorCodeIOPPCS, PolynomialCommitmentScheme, StructuredReferenceString,
};

impl StructuredReferenceString for OrionSRS {
    type PKey = OrionSRS;
    type VKey = OrionSRS;

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        (self.clone(), self.clone())
    }
}

pub struct OrionBaseFieldPCS<F, EvalF, ComPackF, OpenPackF, T>
where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    _marker_f: PhantomData<F>,
    _marker_eval_f: PhantomData<EvalF>,
    _marker_commit_f: PhantomData<ComPackF>,
    _marker_open_f: PhantomData<OpenPackF>,
    _marker_t: PhantomData<T>,
}

impl<F, EvalF, ComPackF, OpenPackF, T> PolynomialCommitmentScheme<EvalF, T>
    for OrionBaseFieldPCS<F, EvalF, ComPackF, OpenPackF, T>
where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    const NAME: &'static str = "OrionBaseFieldPCS";

    type Params = usize;
    type Poly = MultiLinearPoly<F>;
    type EvalPoint = Vec<EvalF>;
    type ScratchPad = OrionScratchPad<F, ComPackF>;

    type SRS = OrionSRS;
    type Commitment = OrionCommitment;
    type Opening = OrionProof<EvalF>;

    fn gen_srs_for_testing(params: &Self::Params, rng: impl rand::RngCore) -> Self::SRS {
        OrionSRS::from_random::<F>(*params, ORION_CODE_PARAMETER_INSTANCE, rng)
    }

    fn init_scratch_pad(_params: &Self::Params) -> Self::ScratchPad {
        OrionScratchPad::default()
    }

    fn commit(
        params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        assert_eq!(*params, proving_key.num_vars);
        orion_commit_base_field(proving_key, poly, scratch_pad).unwrap()
    }

    fn open(
        params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        scratch_pad: &Self::ScratchPad,
        transcript: &mut T,
    ) -> (EvalF, Self::Opening) {
        assert_eq!(*params, proving_key.num_vars);
        orion_open_base_field::<F, EvalF, ComPackF, OpenPackF, T>(
            proving_key,
            poly,
            x,
            transcript,
            scratch_pad,
        )
    }

    fn verify(
        params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: EvalF,
        opening: &Self::Opening,
        transcript: &mut T,
    ) -> bool {
        assert_eq!(*params, verifying_key.num_vars);
        orion_verify_base_field::<F, EvalF, ComPackF, OpenPackF, T>(
            verifying_key,
            commitment,
            x,
            v,
            transcript,
            opening,
        )
    }
}

pub struct OrionSIMDFieldPCS<F, SimdF, EvalF, ComPackF, T>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    _marker_f: PhantomData<F>,
    _marker_simd_f: PhantomData<SimdF>,
    _marker_eval_f: PhantomData<EvalF>,
    _marker_commit_f: PhantomData<ComPackF>,
    _marker_t: PhantomData<T>,
}

impl<F, SimdF, EvalF, ComPackF, T> PolynomialCommitmentScheme<EvalF, T>
    for OrionSIMDFieldPCS<F, SimdF, EvalF, ComPackF, T>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    const NAME: &'static str = "OrionSIMDFieldPCS";

    type Params = usize;
    type Poly = MultiLinearPoly<SimdF>;
    type EvalPoint = Vec<EvalF>;
    type ScratchPad = OrionScratchPad<F, ComPackF>;

    type SRS = OrionSRS;
    type Commitment = OrionCommitment;
    type Opening = OrionProof<EvalF>;

    // NOTE: here we say the number of variables is the sum of 2 following things:
    // - number of variables of the multilinear polynomial
    // - number of variables reside in the SIMD field - e.g., 3 vars for a SIMD 8 field
    fn gen_srs_for_testing(params: &Self::Params, rng: impl rand::RngCore) -> Self::SRS {
        OrionSRS::from_random::<F>(*params, ORION_CODE_PARAMETER_INSTANCE, rng)
    }

    fn init_scratch_pad(_params: &Self::Params) -> Self::ScratchPad {
        OrionScratchPad::default()
    }

    fn commit(
        params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        assert_eq!(*params, proving_key.num_vars);
        assert_eq!(
            poly.get_num_vars(),
            proving_key.num_vars - SimdF::PACK_SIZE.ilog2() as usize
        );
        orion_commit_simd_field(proving_key, poly, scratch_pad).unwrap()
    }

    fn open(
        params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        scratch_pad: &Self::ScratchPad,
        transcript: &mut T,
    ) -> (EvalF, Self::Opening) {
        assert_eq!(*params, proving_key.num_vars);
        assert_eq!(
            poly.get_num_vars(),
            proving_key.num_vars - SimdF::PACK_SIZE.ilog2() as usize
        );
        let opening = orion_open_simd_field::<F, SimdF, EvalF, ComPackF, T>(
            proving_key,
            poly,
            x,
            transcript,
            scratch_pad,
        );

        let num_vars_in_msg = {
            let real_num_vars = poly.get_num_vars() + SimdF::PACK_SIZE.ilog2() as usize;
            let (_, m) = <Self::SRS as TensorCodeIOPPCS>::evals_shape::<F>(real_num_vars);
            m.ilog2() as usize
        };
        let num_vars_in_simd = SimdF::PACK_SIZE.ilog2() as usize;

        // NOTE: working on evaluation response, evaluate the rest of the response
        let mut scratch = vec![EvalF::ZERO; opening.eval_row.len()];
        let eval = MultiLinearPoly::evaluate_with_buffer(
            &opening.eval_row,
            &x[num_vars_in_simd..num_vars_in_simd + num_vars_in_msg],
            &mut scratch,
        );
        drop(scratch);

        (eval, opening)
    }

    fn verify(
        params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: EvalF,
        opening: &Self::Opening,
        transcript: &mut T,
    ) -> bool {
        assert_eq!(*params, verifying_key.num_vars);
        assert_eq!(x.len(), verifying_key.num_vars);
        orion_verify_simd_field::<F, SimdF, EvalF, ComPackF, T>(
            verifying_key,
            commitment,
            x,
            v,
            transcript,
            opening,
        )
    }
}
