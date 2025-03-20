use arith::{Field, SimdField};
use itertools::izip;
use mpi_config::MPIConfig;

use crate::traits::TensorCodeIOPPCS;

use super::{utils::transpose_in_place, OrionCommitment, OrionResult, OrionSRS, OrionScratchPad};

/*
When it comes to MPI assisted multi-process matrix transpose, we can consider the following scenario

p(0):     **************** **************** **************** ... ****************
          elems 0          elems 1          elems 2              elems n - 1

p(1):     **************** **************** **************** ... ****************
          elems 0          elems 1          elems 2              elems n - 1

...

p(n - 1): **************** **************** **************** ... ****************
          elems 0          elems 1          elems 2              elems n - 1

eventually we want the transpose result looking like following:

p(0):     ******************** ******************** ******************** ... ********************
          elems 0 (p(0))       elems 0 (p(1))       elems 0 (p(2))           elems 0 (p(n - 1))

p(1):     ******************** ******************** ******************** ... ********************
          elems 1 (p(0))       elems 1 (p(1))       elems 1 (p(2))           elems 1 (p(n - 1))

...

p(n - 1): ******************** ******************** ******************** ... ********************
          elems n - 1 (p(0))   elems n - 1 (p(1))   elems n - 1 (p(2))       elems n - 1 (p(n - 1))

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

#[allow(unused)]
#[inline(always)]
pub(crate) fn mpi_spread_out_interleaved_alphabets() {}

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
