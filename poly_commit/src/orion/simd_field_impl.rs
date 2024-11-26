use std::iter;
use std::ops::Mul;

use arith::{Field, SimdField};
use polynomials::{EqPolynomial, MultiLinearPoly};
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
    poly: &MultiLinearPoly<SimdF>,
    scratch_pad: &mut OrionScratchPad<F, ComPackF>,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = {
        let num_vars = poly.get_num_vars() + SimdF::PACK_SIZE.ilog2() as usize;
        let (row_field_elems, msg_size) = OrionSRS::evals_shape::<F>(num_vars);
        let row_num = row_field_elems / SimdF::PACK_SIZE;
        (row_num, msg_size)
    };

    let relative_pack_size = ComPackF::PACK_SIZE / SimdF::PACK_SIZE;
    assert_eq!(ComPackF::PACK_SIZE % SimdF::PACK_SIZE, 0);

    let packed_rows = row_num / relative_pack_size;
    assert_eq!(row_num % relative_pack_size, 0);

    let mut evals = poly.coeffs.clone();
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
    let mut scratch = vec![F::ZERO; SimdF::PACK_SIZE * PackF::PACK_SIZE];
    evaluations
        .chunks(PackF::PACK_SIZE)
        .flat_map(|circuit_simd_chunk| -> Vec<PackF> {
            let mut temp: Vec<F> = circuit_simd_chunk.iter().flat_map(|c| c.unpack()).collect();
            transpose_in_place(&mut temp, &mut scratch, PackF::PACK_SIZE);
            temp.chunks(PackF::PACK_SIZE).map(PackF::pack).collect()
        })
        .collect()
}

// NOTE: this implementation doesn't quite align with opening for
// multilinear polynomials over base field,
// as this directly plug into GKR argument system.
// In that context, there is no need to evaluate,
// as evaluation statement can be reduced on the verifier side.
pub fn orion_open_simd_field<F, SimdF, EvalF, SimdEvalF, ComPackF, OpenPackF, T>(
    pk: &OrionSRS,
    poly: &MultiLinearPoly<SimdF>,
    point: &[EvalF],
    transcript: &mut T,
    scratch_pad: &OrionScratchPad<F, ComPackF>,
) -> OrionProof<SimdEvalF>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: Field + From<F> + Mul<F, Output = EvalF>,
    SimdEvalF: SimdField<Scalar = EvalF>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    assert_eq!(SimdF::PACK_SIZE, SimdEvalF::PACK_SIZE);

    let (row_num, msg_size) = {
        let num_vars = poly.get_num_vars() + SimdF::PACK_SIZE.ilog2() as usize;
        assert_eq!(num_vars, point.len());

        let (row_field_elems, msg_size) = OrionSRS::evals_shape::<F>(num_vars);
        let row_num = row_field_elems / SimdF::PACK_SIZE;
        (row_num, msg_size)
    };

    let num_vars_in_row = row_num.ilog2() as usize;
    let num_vars_in_unpacked_msg = point.len() - num_vars_in_row;

    // NOTE: transpose and shuffle evaluations (repack evaluations in another direction)
    // for linear combinations in evaulation/proximity tests
    let mut evals = poly.coeffs.clone();
    assert_eq!(evals.len() * SimdF::PACK_SIZE % OpenPackF::PACK_SIZE, 0);

    let packed_shuffled_evals: Vec<OpenPackF> = transpose_and_shuffle_simd(&mut evals, row_num);
    drop(evals);

    // NOTE: declare the look up tables for column sums
    let tables_num = row_num / OpenPackF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::<EvalF>::new(OpenPackF::PACK_SIZE, tables_num);
    assert_eq!(row_num % OpenPackF::PACK_SIZE, 0);

    // NOTE: working on evaluation response of tensor code IOP based PCS
    let mut eval_row = vec![SimdEvalF::ZERO; msg_size];

    let eq_coeffs = EqPolynomial::build_eq_x_r(&point[num_vars_in_unpacked_msg..]);
    luts.build(&eq_coeffs);

    packed_shuffled_evals
        .chunks(tables_num * SimdEvalF::PACK_SIZE)
        .zip(eval_row.iter_mut())
        .for_each(|(p_col, res)| *res = luts.lookup_and_sum_simd(p_col));

    // NOTE: draw random linear combination out
    // and compose proximity response(s) of tensor code IOP based PCS
    let proximity_test_num = pk.proximity_repetitions::<EvalF>(PCS_SOUNDNESS_BITS);
    let mut proximity_rows = vec![vec![SimdEvalF::ZERO; msg_size]; proximity_test_num];

    proximity_rows.iter_mut().for_each(|row_buffer| {
        let random_coeffs = transcript.generate_challenge_field_elements(row_num);
        luts.build(&random_coeffs);

        packed_shuffled_evals
            .chunks(tables_num * SimdEvalF::PACK_SIZE)
            .zip(row_buffer.iter_mut())
            .for_each(|(p_col, res)| *res = luts.lookup_and_sum_simd(p_col));
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

pub fn orion_verify_simd_field<F, SimdF, EvalF, SimdEvalF, ComPackF, OpenPackF, T>(
    vk: &OrionSRS,
    commitment: &OrionCommitment,
    point: &[EvalF],
    evaluation: EvalF,
    transcript: &mut T,
    proof: &OrionProof<SimdEvalF>,
) -> bool
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: Field + From<F> + Mul<F, Output = EvalF>,
    SimdEvalF: SimdField<Scalar = EvalF>,
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
    let eval_unpacked: Vec<_> = proof.eval_row.iter().flat_map(|e| e.unpack()).collect();
    let mut scratch = vec![EvalF::ZERO; msg_size * SimdEvalF::PACK_SIZE];
    let final_eval = MultiLinearPoly::evaluate_with_buffer(
        &eval_unpacked,
        &point[..num_vars_in_unpacked_msg],
        &mut scratch,
    );
    if final_eval != evaluation {
        return false;
    }

    // NOTE: working on proximity responses, draw random linear combinations
    // then draw query points from fiat shamir transcripts
    let proximity_test_num = vk.proximity_repetitions::<EvalF>(PCS_SOUNDNESS_BITS);
    let random_linear_combinations: Vec<Vec<EvalF>> = (0..proximity_test_num)
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
    let mut scratch = vec![F::ZERO; SimdF::PACK_SIZE * OpenPackF::PACK_SIZE];
    let shuffled_interleaved_alphabet: Vec<Vec<OpenPackF>> = proof
        .query_openings
        .iter()
        .map(|c| -> Vec<_> {
            c.unpack_field_elems::<F, ComPackF>()
                .chunks_mut(SimdF::PACK_SIZE * OpenPackF::PACK_SIZE)
                .flat_map(|circuit_simd_chunk| -> Vec<OpenPackF> {
                    transpose_in_place(circuit_simd_chunk, &mut scratch, OpenPackF::PACK_SIZE);
                    circuit_simd_chunk
                        .chunks(OpenPackF::PACK_SIZE)
                        .map(OpenPackF::pack)
                        .collect()
                })
                .collect()
        })
        .collect();

    // NOTE: declare the look up tables for column sums
    let tables_num = row_num / OpenPackF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::<EvalF>::new(OpenPackF::PACK_SIZE, tables_num);
    assert_eq!(row_num % OpenPackF::PACK_SIZE, 0);

    let eq_linear_combination = EqPolynomial::build_eq_x_r(&point[num_vars_in_unpacked_msg..]);
    random_linear_combinations
        .iter()
        .zip(proof.proximity_rows.iter())
        .chain(iter::once((&eq_linear_combination, &proof.eval_row)))
        .all(|(rl, msg)| {
            let codeword = match vk.code_instance.encode(msg) {
                Ok(c) => c,
                _ => return false,
            };

            luts.build(rl);

            query_indices
                .iter()
                .zip(shuffled_interleaved_alphabet.iter())
                .all(|(&qi, interleaved_alphabet)| {
                    let index = qi % vk.codeword_len();
                    let alphabet: SimdEvalF = luts.lookup_and_sum_simd(interleaved_alphabet);
                    alphabet == codeword[index]
                })
        })
}
