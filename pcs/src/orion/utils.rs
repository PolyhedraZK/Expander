use std::ops::Mul;

use arith::{Field, FieldSerdeError, SimdField};
use thiserror::Error;

/******************************
 * PCS ERROR AND RESULT SETUP *
 ******************************/

#[derive(Debug, Error)]
pub enum OrionPCSError {
    #[error("Orion PCS linear code parameter unmatch error")]
    ParameterUnmatchError,

    #[error("field serde error")]
    SerializationError(#[from] FieldSerdeError),
}

pub type OrionResult<T> = std::result::Result<T, OrionPCSError>;

/****************************************
 * IMPLEMENTATIONS FOR MATRIX TRANSPOSE *
 ****************************************/

pub(crate) const fn cache_batch_size<F: Sized>() -> usize {
    const CACHE_SIZE: usize = 1 << 16;
    CACHE_SIZE / size_of::<F>()
}

#[inline(always)]
pub(crate) fn transpose_in_place<F: Field>(mat: &mut [F], scratch: &mut [F], row_num: usize) {
    let col_num = mat.len() / row_num;
    let batch_size = cache_batch_size::<F>();

    mat.chunks(batch_size)
        .enumerate()
        .for_each(|(i, ith_batch)| {
            let batch_srt = batch_size * i;

            ith_batch.iter().enumerate().for_each(|(j, &elem_j)| {
                let src = batch_srt + j;
                let dst = (src / col_num) + (src % col_num) * row_num;

                scratch[dst] = elem_j;
            })
        });

    mat.copy_from_slice(scratch);
}

#[inline]
pub(crate) fn simd_inner_prod<F0, F1, IPPackF0, IPPackF1>(
    l: &[F0],
    r: &[F1],
    scratch_pl: &mut [IPPackF0],
    scratch_pr: &mut [IPPackF1],
) -> F1
where
    F0: Field,
    F1: Field + From<F0> + Mul<F0, Output = F1>,
    IPPackF0: SimdField<Scalar = F0>,
    IPPackF1: SimdField<Scalar = F1> + Mul<IPPackF0, Output = IPPackF1>,
{
    scratch_pl
        .iter_mut()
        .zip(l.chunks(IPPackF0::PACK_SIZE))
        .for_each(|(pl, ls)| *pl = IPPackF0::pack(ls));

    scratch_pr
        .iter_mut()
        .zip(r.chunks(IPPackF1::PACK_SIZE))
        .for_each(|(pr, rs)| *pr = IPPackF1::pack(rs));

    let simd_sum: IPPackF1 = scratch_pl
        .iter()
        .zip(scratch_pr.iter())
        .map(|(pl, pr)| *pr * *pl)
        .sum();

    simd_sum.unpack().iter().sum()
}
