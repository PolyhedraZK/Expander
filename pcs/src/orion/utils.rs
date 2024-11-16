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

#[allow(unused)]
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

pub(crate) struct SubsetSumLUTs<F: Field> {
    pub entry_bits: usize,
    pub tables: Vec<Vec<F>>,
}

impl<F: Field> SubsetSumLUTs<F> {
    #[inline]
    pub fn new(entry_bits: usize, table_num: usize) -> Self {
        assert!(entry_bits > 0 && table_num > 0);

        Self {
            entry_bits,
            tables: vec![vec![F::ZERO; 1 << entry_bits]; table_num],
        }
    }

    #[inline]
    pub fn build(&mut self, weights: &[F]) {
        assert_eq!(weights.len() % self.entry_bits, 0);
        assert_eq!(weights.len() / self.entry_bits, self.tables.len());

        self.tables.iter_mut().for_each(|lut| lut.fill(F::ZERO));

        // NOTE: we are assuming that the table is for {0, 1}-linear combination
        self.tables
            .iter_mut()
            .zip(weights.chunks(self.entry_bits))
            .for_each(|(lut_i, sub_weights)| {
                sub_weights.iter().enumerate().for_each(|(i, weight_i)| {
                    let bit_mask = 1 << (self.entry_bits - i - 1);
                    lut_i.iter_mut().enumerate().for_each(|(bit_map, li)| {
                        if bit_map & bit_mask == bit_mask {
                            *li += weight_i;
                        }
                    });
                });
            });
    }

    #[inline]
    pub fn lookup_and_sum<BitF, EntryF>(&self, indices: &[EntryF]) -> F
    where
        BitF: Field,
        EntryF: SimdField<Scalar = BitF>,
    {
        // NOTE: at least the entry field elem should have a matching field size
        // and the the entry field being a SIMD field with same packing size as
        // the bits for the table entry
        assert_eq!(EntryF::FIELD_SIZE, 1);
        assert_eq!(EntryF::PACK_SIZE, self.entry_bits);
        assert_eq!(indices.len(), self.tables.len());

        self.tables
            .iter()
            .zip(indices.iter())
            .map(|(t_i, index)| t_i[index.as_u32_unchecked() as usize])
            .sum()
    }
}
