use std::marker::PhantomData;
use std::ops::Mul;

use arith::{Field, SimdField};
use polynomials::MultiLinearPoly;
use transcript::Transcript;

use crate::{
    orion::{OrionCommitment, OrionProof, OrionSRS, OrionScratchPad},
    orion_commit_base_field, orion_open_simd_field, PolynomialCommitmentScheme,
    StructuredReferenceString, ORION_CODE_PARAMETER_INSTANCE,
};

use super::{
    orion_commit_simd_field, orion_open_base_field, orion_verify_base_field,
    orion_verify_simd_field,
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
    EvalF: Field + From<F> + Mul<F, Output = EvalF>,
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
    EvalF: Field + From<F> + Mul<F, Output = EvalF>,
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
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        orion_commit_base_field(proving_key, poly, scratch_pad).unwrap()
    }

    fn open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        scratch_pad: &mut Self::ScratchPad,
        transcript: &mut T,
    ) -> (EvalF, Self::Opening) {
        orion_open_base_field::<F, EvalF, ComPackF, OpenPackF, T>(
            proving_key,
            poly,
            x,
            transcript,
            scratch_pad,
        )
    }

    fn verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: EvalF,
        opening: &Self::Opening,
        transcript: &mut T,
    ) -> bool {
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

pub struct OrionSIMDFieldPCS<F, SimdF, EvalF, SimdEvalF, ComPackF, OpenPackF, T>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: Field + From<F> + Mul<F, Output = EvalF>,
    SimdEvalF: SimdField<Scalar = EvalF>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    _marker_f: PhantomData<F>,
    _marker_simd_f: PhantomData<SimdF>,
    _marker_eval_f: PhantomData<EvalF>,
    _marker_simd_eval_f: PhantomData<SimdEvalF>,
    _marker_commit_f: PhantomData<ComPackF>,
    _marker_open_f: PhantomData<OpenPackF>,
    _marker_t: PhantomData<T>,
}

impl<F, SimdF, EvalF, SimdEvalF, ComPackF, OpenPackF, T> PolynomialCommitmentScheme<EvalF, T>
    for OrionSIMDFieldPCS<F, SimdF, EvalF, SimdEvalF, ComPackF, OpenPackF, T>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: Field + From<F> + Mul<F, Output = EvalF>,
    SimdEvalF: SimdField<Scalar = EvalF>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    const NAME: &'static str = "OrionSIMDFieldPCS";

    type Params = usize;
    type Poly = MultiLinearPoly<SimdF>;
    type EvalPoint = Vec<EvalF>;
    type ScratchPad = OrionScratchPad<F, ComPackF>;

    type SRS = OrionSRS;
    type Commitment = OrionCommitment;
    type Opening = OrionProof<SimdEvalF>;

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
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        orion_commit_simd_field(proving_key, poly, scratch_pad).unwrap()
    }

    fn open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        scratch_pad: &mut Self::ScratchPad,
        transcript: &mut T,
    ) -> (EvalF, Self::Opening) {
        let opening = orion_open_simd_field::<F, SimdF, EvalF, SimdEvalF, ComPackF, OpenPackF, T>(
            proving_key,
            poly,
            x,
            transcript,
            scratch_pad,
        );

        let poly_ext_coeffs: Vec<_> = poly
            .coeffs
            .iter()
            .flat_map(|p| -> Vec<_> { p.unpack().iter().map(|t| EvalF::from(*t)).collect() })
            .collect();

        let mut scratch = vec![EvalF::ZERO; 1 << poly.get_num_vars()];
        let eval = MultiLinearPoly::evaluate_with_buffer(&poly_ext_coeffs, x, &mut scratch);
        drop(scratch);

        (eval, opening)
    }

    fn verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: EvalF,
        opening: &Self::Opening,
        transcript: &mut T,
    ) -> bool {
        orion_verify_simd_field::<F, SimdF, EvalF, SimdEvalF, ComPackF, OpenPackF, T>(
            verifying_key,
            commitment,
            x,
            v,
            transcript,
            opening,
        )
    }
}
