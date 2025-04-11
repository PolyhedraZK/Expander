use arith::{ExtensionField, Field, SimdField};
use gkr_engine::Transcript;
use polynomials::{MultilinearExtension, RefMultiLinearPoly};

use crate::{
    orion::{
        simd_field_impl::orion_commit_simd_field, OrionCommitment, OrionProof, OrionResult,
        OrionSRS, OrionScratchPad,
    },
    orion_open_simd_field,
};

#[inline(always)]
fn pack_from_base<F, PackF>(es: &[F]) -> Vec<PackF>
where
    F: Field,
    PackF: SimdField<Scalar = F>,
{
    // NOTE: SIMD pack neighboring base field evals
    es.chunks(PackF::PACK_SIZE).map(PackF::pack).collect()
}

#[inline(always)]
pub fn orion_commit_base_field<F, SimdF, ComPackF>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<F>,
    scratch_pad: &mut OrionScratchPad,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ComPackF: SimdField<Scalar = F>,
{
    assert_eq!(poly.hypercube_size() % SimdF::PACK_SIZE, 0);
    let packed_evals: Vec<SimdF> = pack_from_base(poly.hypercube_basis_ref());
    let simd_poly = RefMultiLinearPoly::from_ref(&packed_evals);

    orion_commit_simd_field::<F, SimdF, ComPackF>(pk, &simd_poly, scratch_pad)
}

#[inline(always)]
pub fn orion_open_base_field<F, OpenPackF, EvalF, ComPackF>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<F>,
    point: &[EvalF],
    transcript: &mut impl Transcript<EvalF>,
    scratch_pad: &OrionScratchPad,
) -> (EvalF, OrionProof<EvalF>)
where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
{
    assert_eq!(poly.hypercube_size() % OpenPackF::PACK_SIZE, 0);
    let packed_evals: Vec<OpenPackF> = pack_from_base(poly.hypercube_basis_ref());
    let simd_poly = RefMultiLinearPoly::from_ref(&packed_evals);

    orion_open_simd_field::<_, OpenPackF, _, ComPackF>(
        pk,
        &simd_poly,
        point,
        transcript,
        scratch_pad,
    )
}
