use std::collections::VecDeque;

use arith::{ExtensionField, Field, SimdField};
use itertools::izip;
use mpi_config::MPIConfig;
use serdes::ExpSerde;
use transcript::Transcript;
use tree::{Leaf, Node, RangePath, Tree};

use crate::{traits::TensorCodeIOPPCS, PCS_SOUNDNESS_BITS};

use super::{utils::transpose_in_place, OrionCommitment, OrionResult, OrionSRS, OrionScratchPad};

/*
In the Orion scenario, the codeword originally look like following in the RAM:

p(0):    *-*-*-*-*-*-*-*-*-*- .... -*

         *-*-*-*-*-*-*-*-*-*- .... -*

p(1):    *-*-*-*-*-*-*-*-*-*- .... -*

         *-*-*-*-*-*-*-*-*-*- .... -*

...

p(n - 1): *-*-*-*-*-*-*-*-*-*- .... -*

          *-*-*-*-*-*-*-*-*-*- .... -*

a local transpose on each process allows for visiting in interleaved codeword order:

p(0):     * * * * * * * * * *  ....  *
          |/|/|/|/|/|/|/|/|/|  .... /|
          * * * * * * * * * *  ....  *

p(1):     * * * * * * * * * *  ....  *
          |/|/|/|/|/|/|/|/|/|  .... /|
          * * * * * * * * * *  ....  *

...

p(n - 1): * * * * * * * * * *  ....  *
          |/|/|/|/|/|/|/|/|/|  .... /|
          * * * * * * * * * *  ....  *

a global MPI ALL TO ALL rewinds the order into the following:

          p(0)        p(1)        p(2)     p(n - 1)
          * * * *     * * * *     * *  ....  *
          |/|/|/|     |/|/|/|     |/|  .... /|
          * * * *     * * * *     * *  ....  *
               /           /
              /           /
             /           /
            /           /
           /           /           /
          * * * *     * * * *     * *  ....  *
          |/|/|/|     |/|/|/|     |/|  .... /|
          * * * *     * * * *     * *  ....  *
               /           /
              /           /
             /           /
            /           /
           /           /           /
          * * * *     * * * *     * *  ....  *
          |/|/|/|     |/|/|/|     |/|  .... /|
          * * * *     * * * *     * *  ....  *

rearrange each row of interleaved codeword on each process, we have:

          p(0)        p(1)        p(2)     p(n - 1)
          *-*-*-*     *-*-*-*     *-*- .... -*
          *-*-*-*     *-*-*-*     *-*- .... -*
               /           /
              /           /
             /           /
            /           /
           /           /           /
          *-*-*-*     *-*-*-*     *-*- .... -*
          *-*-*-*     *-*-*-*     *-*- .... -*
               /           /
              /           /
             /           /
            /           /
           /           /           /
          *-*-*-*     *-*-*-*     *-*- .... -*
          *-*-*-*     *-*-*-*     *-*- .... -*

eventually, a final transpose each row lead to results of ordering by *WHOLE* interleaved alphabet:

          p(0)                    p(1)                 ....
          *     *     *     *     *     *     *     *
          |    /|    /|    /|     |    /|    /|    /|
          *   / *   / *   / *     *   / *   / *   / *
          |  /  |  /  |  /  |     |  /  |  /  |  /  |
          * /   * /   * /   *     * /   * /   * /   *
          |/    |/    |/    |     |/    |/    |/    |
          *     *     *     *     *     *     *     *

After all these, we can go onwards to MT commitment, and later open alphabets lies in one of the parties.
 */

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

    // NOTE: ALL-TO-ALL transpose go get other world's slice of codeword
    mpi_config.all_to_all_transpose(&mut packed_interleaved_codewords);

    let codeword_po2_len = pk.codeword_len().next_power_of_two();
    let codeword_this_world_len = packed_rows * codeword_po2_len;
    assert_eq!(packed_interleaved_codewords.len(), codeword_this_world_len);

    {
        let codeword_chunk_per_world_len = codeword_this_world_len / mpi_config.world_size();
        assert_eq!(codeword_this_world_len % mpi_config.world_size(), 0);

        let codeword_po2_per_world_len = codeword_po2_len / mpi_config.world_size();
        assert_eq!(codeword_po2_len % mpi_config.world_size(), 0);

        // NOTE: now transpose back to row order of each world's codeword slice
        let mut scratch = vec![PackF::ZERO; codeword_chunk_per_world_len];
        packed_interleaved_codewords
            .chunks_mut(codeword_chunk_per_world_len)
            .for_each(|c| transpose_in_place(c, &mut scratch, codeword_po2_per_world_len));
        drop(scratch);
    }

    // NOTE: transpose back into column order of the codeword slice
    {
        let mut scratch = vec![PackF::ZERO; codeword_this_world_len];
        transpose_in_place(
            &mut packed_interleaved_codewords,
            &mut scratch,
            packed_rows * mpi_config.world_size(),
        );
        drop(scratch);
    }

    scratch_pad.interleaved_alphabet_commitment =
        tree::Tree::compact_new_with_packed_field_elems::<F, PackF>(packed_interleaved_codewords);

    Ok(scratch_pad.interleaved_alphabet_commitment.root())
}

#[inline(always)]
pub(crate) fn mpi_compute_root_merkle_tree<F, PackF>(
    mpi_config: &MPIConfig,
    local_commitment: Node,
    scratch_pad: &mut OrionScratchPad<F, PackF>,
) -> OrionResult<Node>
where
    F: Field,
    PackF: SimdField<Scalar = F>,
{
    let mut leaves = vec![tree::Node::default(); mpi_config.world_size()];
    mpi_config.gather_vec(&vec![local_commitment], &mut leaves);

    {
        let mut leaves_bytes: Vec<u8> = Vec::new();
        if mpi_config.is_root() {
            leaves.serialize_into(&mut leaves_bytes)?;
        }
        mpi_config.root_broadcast_bytes(&mut leaves_bytes);

        if !mpi_config.is_root() {
            leaves = Vec::deserialize_from(leaves_bytes.as_slice())?;
        }
    }

    let root = {
        let height = 1 + leaves.len().ilog2();
        let (internal, leaves) = tree::Tree::new_with_leaf_nodes(leaves, height);
        let dummy_tree = Tree {
            nodes: [internal, leaves].concat(),
            leaves: vec![Leaf::default(); mpi_config.world_size()],
        };

        let dummy_path = dummy_tree.gen_proof(mpi_config.world_rank(), height as usize);
        scratch_pad.path_prefix = dummy_path.path_nodes;

        dummy_tree.root()
    };

    Ok(root)
}

#[allow(unused)]
#[inline(always)]
pub(crate) fn orion_mpi_mt_openings<F, EvalF, PackF, T>(
    mpi_config: &MPIConfig,
    pk: &OrionSRS,
    scratch_pad: &OrionScratchPad<F, PackF>,
    transcript: &mut T,
) -> Option<Vec<RangePath>>
where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    PackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let leaves_in_range_opening = OrionSRS::LEAVES_IN_RANGE_OPENING * mpi_config.world_size();

    // NOTE: MT opening for point queries
    let query_num = pk.query_complexity(PCS_SOUNDNESS_BITS);
    let query_indices: Vec<usize> = {
        let mut indices = transcript.generate_challenge_index_vector(query_num);
        indices.iter_mut().for_each(|q| *q %= pk.codeword_len());
        indices
    };

    let index_range_per_world = pk.codeword_len().next_power_of_two() / mpi_config.world_size();
    let index_starts_this_world = index_range_per_world * mpi_config.world_rank();
    let index_ends_this_world = index_starts_this_world + index_range_per_world;

    let local_paths: Vec<RangePath> = query_indices
        .iter()
        .filter(|&&index| index_starts_this_world <= index && index < index_ends_this_world)
        .map(|index| {
            let left = (index - index_starts_this_world) * leaves_in_range_opening;
            let mut range_opening = scratch_pad
                .interleaved_alphabet_commitment
                .range_query(left, left + leaves_in_range_opening - 1);

            range_opening.prefix_with(&scratch_pad.path_prefix, mpi_config.world_rank());
            range_opening
        })
        .collect();

    let mut global_paths: Vec<Vec<RangePath>> = Vec::new();
    mpi_config.gather_varlen_vec(&local_paths, &mut global_paths);

    if !mpi_config.is_root() {
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
