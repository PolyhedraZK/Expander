use arith::{Field, SimdField};
use polynomials::MultiLinearPoly;

use crate::{orion::utils::transpose_in_place, traits::TensorCodeIOPPCS};

use super::{pcs_impl::commit_encoded, OrionCommitment, OrionResult, OrionSRS, OrionScratchPad};

#[inline(always)]
fn transpose_and_pack_simd<F, CircuitF, PackF>(
    evaluations: &mut [CircuitF],
    row_num: usize,
) -> Vec<PackF>
where
    F: Field,
    CircuitF: SimdField<Scalar = F>,
    PackF: SimdField<Scalar = F>,
{
    // NOTE: pre transpose evaluations
    let mut scratch = vec![CircuitF::ZERO; evaluations.len()];
    transpose_in_place(evaluations, &mut scratch, row_num);
    drop(scratch);

    // NOTE: SIMD pack each row of transposed matrix
    let relative_pack_size = PackF::PACK_SIZE / CircuitF::PACK_SIZE;
    evaluations
        .chunks(relative_pack_size)
        .map(SimdField::pack_from_simd)
        .collect()
}

pub fn orion_commit_simd_field<F, CircuitF, ComPackF>(
    pk: &OrionSRS,
    poly: &MultiLinearPoly<CircuitF>,
    scratch_pad: &mut OrionScratchPad<F, ComPackF>,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    CircuitF: SimdField<Scalar = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = OrionSRS::evals_shape::<CircuitF>(poly.get_num_vars());
    let relative_pack_size = ComPackF::PACK_SIZE / CircuitF::PACK_SIZE;
    assert_eq!(ComPackF::PACK_SIZE % CircuitF::PACK_SIZE, 0);

    let packed_rows = row_num / relative_pack_size;
    assert_eq!(row_num % relative_pack_size, 0);

    let mut evals = poly.coeffs.clone();
    assert_eq!(evals.len() % relative_pack_size, 0);

    let mut packed_evals = transpose_and_pack_simd::<F, CircuitF, ComPackF>(&mut evals, row_num);
    drop(evals);

    // NOTE: transpose back to rows of evaluations, but packed
    let mut scratch = vec![ComPackF::ZERO; packed_rows * msg_size];
    transpose_in_place(&mut packed_evals, &mut scratch, msg_size);
    drop(scratch);

    commit_encoded(pk, &packed_evals, scratch_pad, packed_rows, msg_size)
}
