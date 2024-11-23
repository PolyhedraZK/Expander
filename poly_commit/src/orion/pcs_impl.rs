use std::iter;
use std::ops::Mul;

use arith::{Field, SimdField};
use polynomials::{EqPolynomial, MultiLinearPoly};
use transcript::Transcript;

use crate::{traits::TensorCodeIOPPCS, PCS_SOUNDNESS_BITS};

use super::utils::*;

#[inline(always)]
fn transpose_and_pack<F, PackF>(
    evaluations: &mut [F],
    row_num: usize,
    msg_size: usize,
) -> Vec<PackF>
where
    F: Field,
    PackF: SimdField<Scalar = F>,
{
    // NOTE: pre transpose evaluations
    let mut scratch = vec![F::ZERO; evaluations.len()];
    transpose_in_place(evaluations, &mut scratch, row_num);
    drop(scratch);

    // NOTE: SIMD pack each row of transposed matrix
    assert_eq!(evaluations.len() % PackF::PACK_SIZE, 0);
    let mut packed_evals: Vec<PackF> = evaluations
        .chunks(PackF::PACK_SIZE)
        .map(SimdField::pack)
        .collect();

    // NOTE: transpose back to rows of evaluations, but packed
    let packed_rows = row_num / PackF::PACK_SIZE;
    assert_eq!(row_num % PackF::PACK_SIZE, 0);

    let mut scratch = vec![PackF::ZERO; packed_rows * msg_size];
    transpose_in_place(&mut packed_evals, &mut scratch, msg_size);
    drop(scratch);

    packed_evals
}

#[inline(always)]
fn transpose_and_pack_from_simd<F, CircuitF, PackF>(
    evaluations: &mut [CircuitF],
    row_num: usize,
    msg_size: usize,
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
    assert_eq!(PackF::PACK_SIZE % CircuitF::PACK_SIZE, 0);
    assert_eq!(evaluations.len() % relative_pack_size, 0);
    let mut packed_evals: Vec<PackF> = evaluations
        .chunks(relative_pack_size)
        .map(SimdField::pack_from_simd)
        .collect();

    // NOTE: transpose back to rows of evaluations, but packed
    let packed_rows = row_num / relative_pack_size;
    assert_eq!(row_num % relative_pack_size, 0);

    let mut scratch = vec![PackF::ZERO; packed_rows * msg_size];
    transpose_in_place(&mut packed_evals, &mut scratch, msg_size);
    drop(scratch);

    packed_evals
}

#[inline(always)]
fn commit_encoded<F, PackF>(
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
    packed_evals
        .chunks(msg_size)
        .zip(packed_interleaved_codewords.chunks_mut(pk.codeword_len()))
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

pub fn orion_commit_base<F, ComPackF>(
    pk: &OrionSRS,
    poly: &MultiLinearPoly<F>,
    scratch_pad: &mut OrionScratchPad<F, ComPackF>,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = OrionSRS::evals_shape::<F>(poly.get_num_vars());
    let packed_rows = row_num / ComPackF::PACK_SIZE;
    let mut evals = poly.coeffs.clone();

    let packed_evals = transpose_and_pack(&mut evals, row_num, msg_size);

    commit_encoded(pk, &packed_evals, scratch_pad, packed_rows, msg_size)
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
    let packed_rows = row_num / relative_pack_size;
    let mut evals = poly.coeffs.clone();

    let packed_evals =
        transpose_and_pack_from_simd::<F, CircuitF, ComPackF>(&mut evals, row_num, msg_size);

    commit_encoded(pk, &packed_evals, scratch_pad, packed_rows, msg_size)
}

pub fn orion_open<F, EvalF, ComPackF, OpenPackF, T>(
    pk: &OrionSRS,
    poly: &MultiLinearPoly<F>,
    point: &[EvalF],
    transcript: &mut T,
    scratch_pad: &OrionScratchPad<F, ComPackF>,
) -> (EvalF, OrionProof<EvalF>)
where
    F: Field,
    EvalF: Field + From<F> + Mul<F, Output = EvalF>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let (row_num, msg_size) = OrionSRS::evals_shape::<F>(poly.get_num_vars());
    let num_of_vars_in_msg = msg_size.ilog2() as usize;

    // NOTE: transpose evaluations for linear combinations in evaulation/proximity tests
    let mut transposed_evaluations = poly.coeffs.clone();
    let mut scratch = vec![F::ZERO; 1 << poly.get_num_vars()];
    transpose_in_place(&mut transposed_evaluations, &mut scratch, row_num);
    drop(scratch);

    // NOTE: SIMD pack each row of transposed matrix
    assert_eq!(transposed_evaluations.len() % OpenPackF::PACK_SIZE, 0);
    let packed_evals: Vec<OpenPackF> = transposed_evaluations
        .chunks(OpenPackF::PACK_SIZE)
        .map(OpenPackF::pack)
        .collect();
    drop(transposed_evaluations);

    // NOTE: declare the look up tables for column sums
    let packed_rows = row_num / OpenPackF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::new(OpenPackF::PACK_SIZE, packed_rows);

    // NOTE: working on evaluation response of tensor code IOP based PCS
    let mut eval_row = vec![EvalF::ZERO; msg_size];

    let eq_col_coeffs = EqPolynomial::build_eq_x_r(&point[num_of_vars_in_msg..]);
    luts.build(&eq_col_coeffs);

    packed_evals
        .chunks(packed_rows)
        .zip(eval_row.iter_mut())
        .for_each(|(p_col, res)| *res = luts.lookup_and_sum(p_col));

    // NOTE: draw random linear combination out
    // and compose proximity response(s) of tensor code IOP based PCS
    let proximity_test_num = pk.proximity_repetitions::<EvalF>(PCS_SOUNDNESS_BITS);
    let mut proximity_rows = vec![vec![EvalF::ZERO; msg_size]; proximity_test_num];

    proximity_rows.iter_mut().for_each(|row_buffer| {
        let random_coeffs = transcript.generate_challenge_field_elements(row_num);
        luts.build(&random_coeffs);

        packed_evals
            .chunks(packed_rows)
            .zip(row_buffer.iter_mut())
            .for_each(|(p_col, res)| *res = luts.lookup_and_sum(p_col));
    });
    drop(luts);

    // NOTE: working on evaluation on top of evaluation response
    let mut scratch = vec![EvalF::ZERO; msg_size];
    let eval = MultiLinearPoly::evaluate_with_buffer(
        &eval_row,
        &point[..num_of_vars_in_msg],
        &mut scratch,
    );
    drop(scratch);

    // NOTE: MT opening for point queries
    let leaf_range = row_num / tree::leaf_adic::<F>();
    let query_num = pk.query_complexity(PCS_SOUNDNESS_BITS);
    let query_indices = transcript.generate_challenge_index_vector(query_num);
    let query_openings = query_indices
        .iter()
        .map(|qi| {
            let index = *qi % pk.codeword_len();
            let left = index * leaf_range;
            let right = left + leaf_range - 1;

            scratch_pad
                .interleaved_alphabet_commitment
                .range_query(left, right)
        })
        .collect();

    (
        eval,
        OrionProof {
            eval_row,
            proximity_rows,
            query_openings,
        },
    )
}

pub fn orion_verify<F, EvalF, ComPackF, OpenPackF, T>(
    vk: &OrionSRS,
    commitment: &OrionCommitment,
    point: &[EvalF],
    evaluation: EvalF,
    transcript: &mut T,
    proof: &OrionProof<EvalF>,
) -> bool
where
    F: Field,
    EvalF: Field + From<F> + Mul<F, Output = EvalF>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let (row_num, msg_size) = OrionSRS::evals_shape::<F>(point.len());
    let num_of_vars_in_msg = msg_size.ilog2() as usize;

    // NOTE: working on evaluation response, evaluate the rest of the response
    let mut scratch = vec![EvalF::ZERO; msg_size];
    let final_eval = MultiLinearPoly::evaluate_with_buffer(
        &proof.eval_row,
        &point[..num_of_vars_in_msg],
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
    let leaf_range = row_num / tree::leaf_adic::<F>();
    let mt_consistency =
        query_indices
            .iter()
            .zip(proof.query_openings.iter())
            .all(|(&qi, range_path)| {
                let index = qi % vk.codeword_len();
                range_path.verify(commitment) && index == range_path.left / leaf_range
            });
    if !mt_consistency {
        return false;
    }

    // NOTE: prepare the interleaved alphabets from the MT paths,
    // but pack them back into look up table acceptable formats
    let packed_interleaved_alphabets: Vec<_> = proof
        .query_openings
        .iter()
        .map(|p| -> Vec<_> {
            p.unpack_field_elems::<F, ComPackF>()
                .chunks(OpenPackF::PACK_SIZE)
                .map(OpenPackF::pack)
                .collect()
        })
        .collect();

    // NOTE: encode the proximity/evaluation responses,
    // check againts all challenged indices by check alphabets against
    // linear combined interleaved alphabet
    let mut luts = SubsetSumLUTs::new(OpenPackF::PACK_SIZE, row_num / OpenPackF::PACK_SIZE);
    assert_eq!(row_num % OpenPackF::PACK_SIZE, 0);

    let eq_linear_combination = EqPolynomial::build_eq_x_r(&point[num_of_vars_in_msg..]);
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
                .zip(packed_interleaved_alphabets.iter())
                .all(|(&qi, interleaved_alphabet)| {
                    let index = qi % vk.codeword_len();
                    let alphabet = luts.lookup_and_sum(interleaved_alphabet);
                    alphabet == codeword[index]
                })
        })
}
