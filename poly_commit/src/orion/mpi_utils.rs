use std::collections::VecDeque;

use arith::{ExtensionField, SimdField};
use gkr_engine::{MPIEngine, Transcript};
use itertools::izip;
use serdes::ExpSerde;
use transpose::transpose_inplace;
use tree::{RangePath, Tree};

use crate::{
    orion::{OrionCommitment, OrionResult, OrionSRS, OrionScratchPad},
    traits::TensorCodeIOPPCS,
    utils::mpi_matrix_transpose,
    PCS_SOUNDNESS_BITS,
};

#[inline(always)]
pub(crate) fn mpi_commit_encoded<PackF>(
    mpi_engine: &impl MPIEngine,
    pk: &OrionSRS,
    packed_evals: &[PackF],
    scratch_pad: &mut OrionScratchPad,
) -> OrionResult<OrionCommitment>
where
    PackF: SimdField,
{
    let packed_rows = pk.local_num_fs_per_query() / PackF::PACK_SIZE;

    // NOTE: packed codeword buffer and encode over packed field
    let codeword_len_po2 = pk.codeword_len().next_power_of_two();
    let mut codewords = vec![PackF::ZERO; packed_rows * codeword_len_po2];
    izip!(
        packed_evals.chunks(pk.message_len()),
        codewords.chunks_mut(codeword_len_po2)
    )
    .try_for_each(|(evals, codeword)| pk.code_instance.encode_in_place(evals, codeword))?;

    if packed_rows > 1 {
        let mut scratch = vec![PackF::ZERO; std::cmp::max(packed_rows, codeword_len_po2)];
        transpose_inplace(&mut codewords, &mut scratch, codeword_len_po2, packed_rows);
        drop(scratch)
    }

    mpi_matrix_transpose(mpi_engine, &mut codewords, packed_rows);

    scratch_pad.interleaved_alphabet_commitment =
        Tree::compact_new_with_packed_field_elems(codewords);

    // NOTE: gather local roots and compute the final MT root
    let local_commitment = scratch_pad.interleaved_alphabet_commitment.root();
    let mut leaves = vec![tree::Node::default(); mpi_engine.world_size()];
    mpi_engine.gather_vec(&[local_commitment], &mut leaves);

    {
        let mut leaves_bytes: Vec<u8> = Vec::new();
        leaves.serialize_into(&mut leaves_bytes)?;
        mpi_engine.root_broadcast_bytes(&mut leaves_bytes);

        if !mpi_engine.is_root() {
            leaves = Vec::deserialize_from(leaves_bytes.as_slice())?;
        }
    }

    scratch_pad.merkle_cap = leaves.clone();

    let root = {
        let height = 1 + leaves.len().ilog2();
        let internal = tree::Tree::new_with_leaf_nodes(&leaves, height);
        internal[0]
    };

    Ok(root)
}

#[inline(always)]
pub(crate) fn orion_mpi_mt_openings<EvalF, T>(
    mpi_engine: &impl MPIEngine,
    pk: &OrionSRS,
    scratch_pad: &OrionScratchPad,
    transcript: &mut T,
) -> Option<Vec<RangePath>>
where
    EvalF: ExtensionField,
    T: Transcript<EvalF>,
{
    let num_leaves_per_opening = pk.num_leaves_per_mt_query();

    // NOTE: MT opening for point queries
    let query_num = pk.query_complexity(PCS_SOUNDNESS_BITS);
    let query_indices: Vec<usize> = {
        let mut indices = transcript.generate_challenge_index_vector(query_num);
        indices.iter_mut().for_each(|q| *q %= pk.codeword_len());
        indices
    };

    let index_range_per_world = pk.codeword_len().next_power_of_two() / mpi_engine.world_size();
    let index_starts_this_world = index_range_per_world * mpi_engine.world_rank();
    let index_ends_this_world = index_starts_this_world + index_range_per_world;

    let local_paths: Vec<RangePath> = query_indices
        .iter()
        .filter(|&&index| index_starts_this_world <= index && index < index_ends_this_world)
        .map(|index| {
            let left = (index - index_starts_this_world) * num_leaves_per_opening;
            scratch_pad
                .interleaved_alphabet_commitment
                .range_query(left, left + num_leaves_per_opening - 1)
        })
        .collect();

    let mut global_paths: Vec<Vec<RangePath>> = Vec::new();
    mpi_engine.gather_varlen_vec(&local_paths, &mut global_paths);

    if !mpi_engine.is_root() {
        return None;
    }

    let mut global_paths_deque: Vec<VecDeque<RangePath>> =
        global_paths.into_iter().map(VecDeque::from).collect();

    let flattened_paths: Vec<RangePath> = query_indices
        .iter()
        .map(|q| {
            let which_world = q / index_range_per_world;
            global_paths_deque[which_world].pop_front().unwrap()
        })
        .collect();

    flattened_paths.into()
}
