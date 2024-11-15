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

/*********************
 * LINEAR OPERATIONS *
 *********************/

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

pub(crate) struct LookupTables<F: Field> {
    pub table_bits: usize,
    pub table_num: usize,
    pub tables: Vec<Vec<F>>,
}

impl<F: Field> LookupTables<F> {
    #[inline]
    pub fn new(table_bits: usize, table_num: usize) -> Self {
        Self {
            table_bits,
            table_num,
            tables: vec![vec![F::ZERO; 1 << table_bits]; table_num],
        }
    }

    #[inline]
    pub fn build(&mut self, weights: &[F]) {
        assert_eq!(weights.len() % self.table_bits, 0);
        assert_eq!(weights.len() / self.table_bits, self.table_num);

        self.zeroize();

        self.tables
            .iter_mut()
            .zip(weights.chunks(self.table_bits))
            .for_each(|(lut_i, sub_weights)| {
                sub_weights.iter().enumerate().for_each(|(i, weight_i)| {
                    let bit_mask = 1 << (self.table_bits - i - 1);
                    lut_i.iter_mut().enumerate().for_each(|(bit_map, li)| {
                        if bit_map & bit_mask == bit_mask {
                            *li += weight_i;
                        }
                    });
                });
            });
    }

    #[inline]
    pub fn zeroize(&mut self) {
        self.tables.iter_mut().for_each(|lut| lut.fill(F::ZERO));
    }

    #[inline]
    pub fn lookup_and_sum<F0: Field>(&self, indices: &[F0]) -> F {
        self.tables
            .iter()
            .zip(indices.iter())
            .map(|(t_i, index)| t_i[index.as_u32_unchecked() as usize])
            .sum()
    }
}
