use arith::{Field, SimdField};
use itertools::izip;
use mpi_config::MPIConfig;

use crate::traits::TensorCodeIOPPCS;

use super::{utils::transpose_in_place, OrionCommitment, OrionResult, OrionSRS, OrionScratchPad};

#[allow(unused)]
#[inline(always)]
pub(crate) fn mpi_commit_encoded<F, PackF>(
    mpi_config: &MPIConfig,
    pk: &OrionSRS,
    packed_evals: &[PackF],
    scratch_pad: &mut OrionScratchPad<F, PackF>,
    packed_rows: usize,
    msg_size: usize,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    PackF: SimdField<Scalar = F>,
{
    // NOTE: packed codeword buffer and encode over packed field
    let mut packed_interleaved_codewords = vec![PackF::ZERO; packed_rows * pk.codeword_len()];
    izip!(
        packed_evals.chunks(msg_size),
        packed_interleaved_codewords.chunks_mut(pk.codeword_len())
    )
    .try_for_each(|(evals, codeword)| pk.code_instance.encode_in_place(evals, codeword))?;

    // NOTE: transpose codeword s.t., the matrix has codewords being columns
    let mut scratch = vec![PackF::ZERO; packed_rows * pk.codeword_len()];
    transpose_in_place(&mut packed_interleaved_codewords, &mut scratch, packed_rows);
    drop(scratch);

    // NOTE: commit the interleaved codeword
    // we just directly commit to the packed field elements to leaves
    // Also note, when codeword is not power of 2 length, pad to nearest po2
    // to commit by merkle tree
    if !packed_interleaved_codewords.len().is_power_of_two() {
        let aligned_po2_len = packed_interleaved_codewords.len().next_power_of_two();
        packed_interleaved_codewords.resize(aligned_po2_len, PackF::ZERO);
    }
    scratch_pad.interleaved_alphabet_commitment =
        tree::Tree::compact_new_with_packed_field_elems::<F, PackF>(packed_interleaved_codewords);

    Ok(scratch_pad.interleaved_alphabet_commitment.root())
}
