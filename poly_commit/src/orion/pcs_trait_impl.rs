use std::{marker::PhantomData, ops::Mul};

use arith::{ExtensionField, Field, SimdField};
use gkr_engine::{StructuredReferenceString, Transcript};
use polynomials::MultiLinearPoly;

use crate::{orion::*, traits::TensorCodeIOPPCS, PolynomialCommitmentScheme};

impl StructuredReferenceString for OrionSRS {
    type PKey = OrionSRS;
    type VKey = OrionSRS;

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        (self.clone(), self.clone())
    }
}

pub struct OrionBaseFieldPCS<SimdEvalF, ComPackF, OpenPackF, T>
where
    SimdEvalF: SimdField + ExtensionField,
    SimdEvalF::BaseField: SimdField + Mul<OpenPackF, Output = SimdEvalF::BaseField>,
    SimdEvalF::Scalar: ExtensionField<BaseField = <SimdEvalF::BaseField as SimdField>::Scalar>,
    ComPackF: SimdField,
    OpenPackF: SimdField<Scalar = ComPackF::Scalar>,
    T: Transcript,
{
    _marker_: PhantomData<(SimdEvalF, ComPackF, OpenPackF, T)>,
}

impl<SimdEvalF, ComPackF, OpenPackF, T> PolynomialCommitmentScheme<SimdEvalF::Scalar>
    for OrionBaseFieldPCS<SimdEvalF, ComPackF, OpenPackF, T>
where
    SimdEvalF: SimdField + ExtensionField,
    SimdEvalF::BaseField: SimdField + Mul<OpenPackF, Output = SimdEvalF::BaseField>,
    SimdEvalF::Scalar: ExtensionField<BaseField = <SimdEvalF::BaseField as SimdField>::Scalar>,
    ComPackF: SimdField,
    OpenPackF: SimdField<Scalar = ComPackF::Scalar>,
    T: Transcript,
{
    const NAME: &'static str = "OrionBaseFieldPCS";

    type Params = usize;
    type Poly = MultiLinearPoly<ComPackF::Scalar>;
    type EvalPoint = Vec<SimdEvalF::Scalar>;
    type ScratchPad = OrionScratchPad<ComPackF>;

    type SRS = OrionSRS;
    type Commitment = OrionCommitment;
    type Opening = OrionProof<SimdEvalF::Scalar>;

    const MINIMUM_NUM_VARS: usize =
        (Self::SRS::LEAVES_IN_RANGE_OPENING * tree::leaf_adic::<ComPackF::Scalar>()).ilog2() as usize;

    fn gen_srs_for_testing(params: &Self::Params, rng: impl rand::RngCore) -> Self::SRS {
        OrionSRS::from_random::<ComPackF::Scalar>(*params, ORION_CODE_PARAMETER_INSTANCE, rng)
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
        transcript: &mut impl Transcript,
    ) -> (SimdEvalF::Scalar, Self::Opening) {
        assert_eq!(*params, proving_key.num_vars);
        orion_open_base_field::<SimdEvalF, ComPackF, OpenPackF>(
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
        v: SimdEvalF::Scalar,
        opening: &Self::Opening,
        transcript: &mut impl Transcript,
    ) -> bool {
        assert_eq!(*params, verifying_key.num_vars);
        orion_verify_base_field::<SimdEvalF, ComPackF, OpenPackF>(
            verifying_key,
            commitment,
            x,
            v,
            transcript,
            opening,
        )
    }
}

pub struct OrionSIMDFieldPCS<SimdPolyF, SimdEvalF, ComPackF>
where
    SimdPolyF: SimdField,
    SimdEvalF: SimdField + ExtensionField,
    SimdEvalF::BaseField: SimdField + Mul<SimdPolyF, Output = SimdEvalF::BaseField>,
    SimdEvalF::Scalar: ExtensionField<BaseField = <SimdEvalF::BaseField as SimdField>::Scalar>,
    ComPackF: SimdField<Scalar = SimdPolyF::Scalar>,
{
    _marker_: PhantomData<(SimdPolyF, SimdEvalF, ComPackF)>,
}

impl<SimdF, SimdEvalF, ComPackF> PolynomialCommitmentScheme<SimdEvalF::Scalar>
    for OrionSIMDFieldPCS<SimdF, SimdEvalF, ComPackF>
where
    SimdF: SimdField,
    SimdEvalF: SimdField + ExtensionField,
    SimdEvalF::BaseField: SimdField + Mul<SimdF, Output = SimdEvalF::BaseField>,
    SimdEvalF::Scalar: ExtensionField<BaseField = <SimdEvalF::BaseField as SimdField>::Scalar>,
    ComPackF: SimdField<Scalar = SimdF::Scalar>,
{
    const NAME: &'static str = "OrionSIMDFieldPCS";

    type Params = usize;
    type Poly = MultiLinearPoly<SimdF>;
    type EvalPoint = Vec<SimdEvalF::Scalar>;
    type ScratchPad = OrionScratchPad<ComPackF>;

    type SRS = OrionSRS;
    type Commitment = OrionCommitment;
    type Opening = OrionProof<SimdEvalF::Scalar>;

    const MINIMUM_NUM_VARS: usize = (Self::SRS::LEAVES_IN_RANGE_OPENING * tree::leaf_adic::<SimdF::Scalar>()
        / SimdF::PACK_SIZE)
        .ilog2() as usize;

    // NOTE: here we say the number of variables is the sum of 2 following things:
    // - number of variables of the multilinear polynomial
    // - number of variables reside in the SIMD field - e.g., 3 vars for a SIMD 8 field
    fn gen_srs_for_testing(params: &Self::Params, rng: impl rand::RngCore) -> Self::SRS {
        OrionSRS::from_random::<SimdF::Scalar>(*params, ORION_CODE_PARAMETER_INSTANCE, rng)
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
        transcript: &mut impl Transcript,
    ) -> (SimdEvalF::Scalar, Self::Opening) {
        assert_eq!(*params, proving_key.num_vars);
        assert_eq!(
            poly.get_num_vars(),
            proving_key.num_vars - SimdF::PACK_SIZE.ilog2() as usize
        );
        let opening = orion_open_simd_field::<SimdF, SimdEvalF, ComPackF>(
            proving_key,
            poly,
            x,
            transcript,
            scratch_pad,
        );

        let num_vars_in_msg = {
            let real_num_vars = poly.get_num_vars() + SimdF::PACK_SIZE.ilog2() as usize;
            let (_, m) = <Self::SRS as TensorCodeIOPPCS>::evals_shape::<SimdF::Scalar>(real_num_vars);
            m.ilog2() as usize
        };
        let num_vars_in_com_simd = ComPackF::PACK_SIZE.ilog2() as usize;

        // NOTE: working on evaluation response, evaluate the rest of the response
        let mut scratch = vec![SimdEvalF::Scalar::ZERO; opening.eval_row.len()];
        let eval = MultiLinearPoly::evaluate_with_buffer(
            &opening.eval_row,
            &x[num_vars_in_com_simd..num_vars_in_com_simd + num_vars_in_msg],
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
        v: SimdEvalF::Scalar,
        opening: &Self::Opening,
        transcript: &mut impl Transcript,
    ) -> bool {
        assert_eq!(*params, verifying_key.num_vars);
        assert_eq!(x.len(), verifying_key.num_vars);
        orion_verify_simd_field::<SimdF, SimdEvalF, ComPackF>(
            verifying_key,
            commitment,
            x,
            v,
            transcript,
            opening,
        )
    }
}
