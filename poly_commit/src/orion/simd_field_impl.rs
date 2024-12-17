use std::iter;

use arith::{ExtensionField, Field, SimdField};
use itertools::izip;
use polynomials::{EqPolynomial, MultilinearExtension, RefMultiLinearPoly};
use transcript::Transcript;

use crate::{
    orion::{
        utils::{commit_encoded, orion_mt_openings, orion_mt_verify, transpose_in_place},
        OrionCommitment, OrionProof, OrionResult, OrionSRS, OrionScratchPad,
    },
    traits::TensorCodeIOPPCS,
    SubsetSumLUTs, PCS_SOUNDNESS_BITS,
};

#[inline(always)]
fn transpose_and_pack_simd<F, SimdF, PackF>(evaluations: &mut [SimdF], row_num: usize) -> Vec<PackF>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    PackF: SimdField<Scalar = F>,
{
    // NOTE: pre transpose evaluations
    let mut scratch = vec![SimdF::ZERO; evaluations.len()];
    transpose_in_place(evaluations, &mut scratch, row_num);
    drop(scratch);

    // NOTE: SIMD pack each row of transposed matrix
    let relative_pack_size = PackF::PACK_SIZE / SimdF::PACK_SIZE;
    evaluations
        .chunks(relative_pack_size)
        .map(PackF::pack_from_simd)
        .collect()
}

pub fn orion_commit_simd_field<F, SimdF, ComPackF>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<SimdF>,
    scratch_pad: &mut OrionScratchPad<F, ComPackF>,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = {
        let num_vars = poly.num_vars() + SimdF::PACK_SIZE.ilog2() as usize;
        let (row_field_elems, msg_size) = OrionSRS::evals_shape::<F>(num_vars);
        let row_num = row_field_elems / SimdF::PACK_SIZE;
        (row_num, msg_size)
    };

    let relative_pack_size = ComPackF::PACK_SIZE / SimdF::PACK_SIZE;
    assert_eq!(ComPackF::PACK_SIZE % SimdF::PACK_SIZE, 0);

    let packed_rows = row_num / relative_pack_size;
    assert_eq!(row_num % relative_pack_size, 0);

    let mut evals = poly.hypercube_basis();
    assert_eq!(evals.len() % relative_pack_size, 0);

    let mut packed_evals = transpose_and_pack_simd::<F, SimdF, ComPackF>(&mut evals, row_num);
    drop(evals);

    // NOTE: transpose back to rows of evaluations, but packed
    let mut scratch = vec![ComPackF::ZERO; packed_rows * msg_size];
    transpose_in_place(&mut packed_evals, &mut scratch, msg_size);
    drop(scratch);

    commit_encoded(pk, &packed_evals, scratch_pad, packed_rows, msg_size)
}

#[inline(always)]
fn transpose_and_shuffle_simd<F, SimdF, PackF>(
    evaluations: &mut [SimdF],
    row_num: usize,
) -> Vec<PackF>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    PackF: SimdField<Scalar = F>,
{
    // NOTE: pre transpose evaluations
    let mut scratch = vec![SimdF::ZERO; evaluations.len()];
    transpose_in_place(evaluations, &mut scratch, row_num);
    drop(scratch);

    // NOTE: reshuffle the transposed matrix, from SIMD over row to SIMD over col
    let mut scratch = vec![F::ZERO; SimdF::PACK_SIZE * row_num];
    evaluations
        .chunks(row_num)
        .flat_map(|row_simds| -> Vec<_> {
            let mut elts: Vec<_> = row_simds.iter().flat_map(|f| f.unpack()).collect();
            transpose_in_place(&mut elts, &mut scratch, row_num);
            elts.chunks(PackF::PACK_SIZE).map(PackF::pack).collect()
        })
        .collect()
}

// NOTE: this implementation doesn't quite align with opening for
// multilinear polynomials over base field,
// as this directly plug into GKR argument system.
// In that context, there is no need to evaluate,
// as evaluation statement can be reduced on the verifier side.
pub fn orion_open_simd_field<F, SimdF, EvalF, ComPackF, OpenPackF, T>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<SimdF>,
    point: &[EvalF],
    transcript: &mut T,
    scratch_pad: &OrionScratchPad<F, ComPackF>,
) -> OrionProof<EvalF>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let (row_num, msg_size) = {
        let num_vars = poly.num_vars() + SimdF::PACK_SIZE.ilog2() as usize;
        assert_eq!(num_vars, point.len());

        let (row_field_elems, msg_size) = OrionSRS::evals_shape::<F>(num_vars);
        let row_num = row_field_elems / SimdF::PACK_SIZE;
        (row_num, msg_size)
    };

    let num_vars_in_row = row_num.ilog2() as usize;
    let num_vars_in_unpacked_msg = point.len() - num_vars_in_row;

    // NOTE: transpose and shuffle evaluations (repack evaluations in another direction)
    // for linear combinations in evaulation/proximity tests
    let mut evals = poly.hypercube_basis();
    assert_eq!(evals.len() * SimdF::PACK_SIZE % OpenPackF::PACK_SIZE, 0);

    let packed_shuffled_evals: Vec<OpenPackF> = transpose_and_shuffle_simd(&mut evals, row_num);
    drop(evals);

    // NOTE: declare the look up tables for column sums
    let tables_num = row_num / OpenPackF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::<EvalF>::new(OpenPackF::PACK_SIZE, tables_num);
    assert_eq!(row_num % OpenPackF::PACK_SIZE, 0);

    // NOTE: working on evaluation response of tensor code IOP based PCS
    let mut eval_row = vec![EvalF::ZERO; msg_size * SimdF::PACK_SIZE];

    let eq_coeffs = EqPolynomial::build_eq_x_r(&point[num_vars_in_unpacked_msg..]);
    luts.build(&eq_coeffs);

    izip!(packed_shuffled_evals.chunks(tables_num), &mut eval_row)
        .for_each(|(p_col, res)| *res = luts.lookup_and_sum(p_col));

    // NOTE: draw random linear combination out
    // and compose proximity response(s) of tensor code IOP based PCS
    let proximity_test_num = pk.proximity_repetitions::<EvalF>(PCS_SOUNDNESS_BITS);
    let mut proximity_rows =
        vec![vec![EvalF::ZERO; msg_size * SimdF::PACK_SIZE]; proximity_test_num];

    proximity_rows.iter_mut().for_each(|row_buffer| {
        let random_coeffs = transcript.generate_challenge_field_elements(row_num);
        luts.build(&random_coeffs);

        izip!(packed_shuffled_evals.chunks(tables_num), row_buffer)
            .for_each(|(p_col, res)| *res = luts.lookup_and_sum(p_col));
    });
    drop(luts);

    // NOTE: MT opening for point queries
    let query_openings = orion_mt_openings(pk, transcript, scratch_pad);

    OrionProof {
        eval_row,
        proximity_rows,
        query_openings,
    }
}

pub fn orion_verify_simd_field<F, SimdF, EvalF, ComPackF, OpenPackF, T>(
    vk: &OrionSRS,
    commitment: &OrionCommitment,
    point: &[EvalF],
    evaluation: EvalF,
    transcript: &mut T,
    proof: &OrionProof<EvalF>,
) -> bool
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let (row_num, msg_size) = {
        let (row_field_elems, msg_size) = OrionSRS::evals_shape::<F>(point.len());
        let row_num = row_field_elems / SimdF::PACK_SIZE;
        (row_num, msg_size)
    };

    let num_vars_in_row = row_num.ilog2() as usize;
    let num_vars_in_unpacked_msg = point.len() - num_vars_in_row;

    // NOTE: working on evaluation response, evaluate the rest of the response
    let mut scratch = vec![EvalF::ZERO; msg_size * SimdF::PACK_SIZE];
    let final_eval = RefMultiLinearPoly::from_ref(&proof.eval_row)
        .evaluate_with_buffer(&point[..num_vars_in_unpacked_msg], &mut scratch);

    if final_eval != evaluation {
        return false;
    }

    // NOTE: working on proximity responses, draw random linear combinations
    // then draw query points from fiat shamir transcripts
    let proximity_reps = vk.proximity_repetitions::<EvalF>(PCS_SOUNDNESS_BITS);
    let random_linear_combinations: Vec<Vec<EvalF>> = (0..proximity_reps)
        .map(|_| transcript.generate_challenge_field_elements(row_num))
        .collect();
    let query_num = vk.query_complexity(PCS_SOUNDNESS_BITS);
    let query_indices = transcript.generate_challenge_index_vector(query_num);

    // NOTE: check consistency in MT in the opening trees and against the commitment tree
    if !orion_mt_verify(vk, &query_indices, &proof.query_openings, commitment) {
        return false;
    }

    // NOTE: prepare the interleaved alphabets from the MT paths,
    // but reshuffle the packed elements into another direction
    let mut scratch = vec![F::ZERO; SimdF::PACK_SIZE * row_num];
    let shuffled_interleaved_alphabet: Vec<Vec<OpenPackF>> = proof
        .query_openings
        .iter()
        .map(|c| -> Vec<_> {
            let mut elts = c.unpack_field_elems::<F, ComPackF>();
            transpose_in_place(&mut elts, &mut scratch, row_num);
            elts.chunks(OpenPackF::PACK_SIZE)
                .map(OpenPackF::pack)
                .collect()
        })
        .collect();

    // NOTE: declare the look up tables for column sums
    let tables_num = row_num / OpenPackF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::<EvalF>::new(OpenPackF::PACK_SIZE, tables_num);
    assert_eq!(row_num % OpenPackF::PACK_SIZE, 0);

    let eq_linear_combination = EqPolynomial::build_eq_x_r(&point[num_vars_in_unpacked_msg..]);
    let mut scratch_msg = vec![EvalF::ZERO; SimdF::PACK_SIZE * msg_size];
    let mut scratch_codeword = vec![EvalF::ZERO; SimdF::PACK_SIZE * vk.codeword_len()];

    izip!(&random_linear_combinations, &proof.proximity_rows)
        .chain(iter::once((&eq_linear_combination, &proof.eval_row)))
        .all(|(rl, msg)| {
            let mut msg_cloned = msg.clone();
            transpose_in_place(&mut msg_cloned, &mut scratch_msg, msg_size);
            let mut codeword: Vec<_> = msg_cloned
                .chunks(msg_size)
                .flat_map(|m| vk.code_instance.encode(m).unwrap())
                .collect();
            transpose_in_place(&mut codeword, &mut scratch_codeword, SimdF::PACK_SIZE);

            luts.build(rl);

            izip!(&query_indices, &shuffled_interleaved_alphabet).all(
                |(&qi, interleaved_alphabet)| {
                    let index = qi % vk.codeword_len();

                    let simd_starts = index * SimdF::PACK_SIZE;
                    let simd_ends = (index + 1) * SimdF::PACK_SIZE;

                    izip!(
                        &codeword[simd_starts..simd_ends],
                        interleaved_alphabet.chunks(tables_num)
                    )
                    .all(|(expected_alphabet, packed_index)| {
                        let alphabet = luts.lookup_and_sum(packed_index);
                        alphabet == *expected_alphabet
                    })
                },
            )
        })
}
