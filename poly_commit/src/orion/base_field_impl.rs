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
fn transpose_and_pack<F, PackF>(evaluations: &mut [F], row_num: usize) -> Vec<PackF>
where
    F: Field,
    PackF: SimdField<Scalar = F>,
{
    // NOTE: pre transpose evaluations
    let mut scratch = vec![F::ZERO; evaluations.len()];
    transpose_in_place(evaluations, &mut scratch, row_num);
    drop(scratch);

    // NOTE: SIMD pack each row of transposed matrix
    evaluations
        .chunks(PackF::PACK_SIZE)
        .map(SimdField::pack)
        .collect()
}

pub fn orion_commit_base_field<F, ComPackF>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<F>,
    scratch_pad: &mut OrionScratchPad<F, ComPackF>,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = OrionSRS::evals_shape::<F>(poly.num_vars());
    let packed_rows = row_num / ComPackF::PACK_SIZE;
    assert_eq!(row_num % ComPackF::PACK_SIZE, 0);

    let mut evals = poly.hypercube_basis();
    assert_eq!(evals.len() % ComPackF::PACK_SIZE, 0);

    let mut packed_evals: Vec<ComPackF> = transpose_and_pack(&mut evals, row_num);
    drop(evals);

    // NOTE: transpose back to rows of evaluations, but packed
    let mut scratch = vec![ComPackF::ZERO; packed_rows * msg_size];
    transpose_in_place(&mut packed_evals, &mut scratch, msg_size);
    drop(scratch);

    commit_encoded(pk, &packed_evals, scratch_pad, packed_rows, msg_size)
}

pub fn orion_open_base_field<F, EvalF, ComPackF, OpenPackF, T>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<F>,
    point: &[EvalF],
    transcript: &mut T,
    scratch_pad: &OrionScratchPad<F, ComPackF>,
) -> (EvalF, OrionProof<EvalF>)
where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let (row_num, msg_size) = OrionSRS::evals_shape::<F>(poly.num_vars());
    let num_vars_in_row = row_num.ilog2() as usize;

    // NOTE: transpose evaluations for linear combinations in evaulation/proximity tests
    let mut evals = poly.hypercube_basis();
    assert_eq!(evals.len() % OpenPackF::PACK_SIZE, 0);

    let packed_evals: Vec<OpenPackF> = transpose_and_pack(&mut evals, row_num);
    drop(evals);

    // NOTE: declare the look up tables for column sums
    let table_num = row_num / OpenPackF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::<EvalF>::new(OpenPackF::PACK_SIZE, table_num);
    assert_eq!(row_num % OpenPackF::PACK_SIZE, 0);

    // NOTE: working on evaluation response of tensor code IOP based PCS
    let mut eval_row = vec![EvalF::ZERO; msg_size];

    let eq_col_coeffs = EqPolynomial::build_eq_x_r(&point[point.len() - num_vars_in_row..]);
    luts.build(&eq_col_coeffs);

    izip!(packed_evals.chunks(table_num), &mut eval_row)
        .for_each(|(p_col, res)| *res = luts.lookup_and_sum(p_col));

    // NOTE: draw random linear combination out
    // and compose proximity response(s) of tensor code IOP based PCS
    let proximity_test_num = pk.proximity_repetitions::<EvalF>(PCS_SOUNDNESS_BITS);
    let mut proximity_rows = vec![vec![EvalF::ZERO; msg_size]; proximity_test_num];

    proximity_rows.iter_mut().for_each(|row_buffer| {
        let random_coeffs = transcript.generate_challenge_field_elements(row_num);
        luts.build(&random_coeffs);

        izip!(packed_evals.chunks(table_num), row_buffer)
            .for_each(|(p_col, res)| *res = luts.lookup_and_sum(p_col));
    });
    drop(luts);

    // NOTE: working on evaluation on top of evaluation response
    let mut scratch = vec![EvalF::ZERO; msg_size];
    let eval = RefMultiLinearPoly::from_ref(&eval_row)
        .evaluate_with_buffer(&point[..point.len() - num_vars_in_row], &mut scratch);
    drop(scratch);

    // NOTE: MT opening for point queries
    let query_openings = orion_mt_openings(pk, transcript, scratch_pad);

    (
        eval,
        OrionProof {
            eval_row,
            proximity_rows,
            query_openings,
        },
    )
}

pub fn orion_verify_base_field<F, EvalF, ComPackF, OpenPackF, T>(
    vk: &OrionSRS,
    commitment: &OrionCommitment,
    point: &[EvalF],
    evaluation: EvalF,
    transcript: &mut T,
    proof: &OrionProof<EvalF>,
) -> bool
where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let (row_num, msg_size) = OrionSRS::evals_shape::<F>(point.len());
    let num_vars_in_msg = msg_size.ilog2() as usize;

    // NOTE: working on evaluation response, evaluate the rest of the response
    let mut scratch = vec![EvalF::ZERO; msg_size];
    let final_eval = RefMultiLinearPoly::from_ref(&proof.eval_row)
        .evaluate_with_buffer(&point[..num_vars_in_msg], &mut scratch);

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
    let tables_num = row_num / OpenPackF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::<EvalF>::new(OpenPackF::PACK_SIZE, tables_num);
    assert_eq!(row_num % OpenPackF::PACK_SIZE, 0);

    let eq_linear_combination = EqPolynomial::build_eq_x_r(&point[num_vars_in_msg..]);

    izip!(&random_linear_combinations, &proof.proximity_rows)
        .chain(iter::once((&eq_linear_combination, &proof.eval_row)))
        .all(|(rl, msg)| {
            let codeword = match vk.code_instance.encode(msg) {
                Ok(c) => c,
                _ => return false,
            };

            luts.build(rl);

            izip!(&query_indices, &packed_interleaved_alphabets).all(
                |(qi, interleaved_alphabet)| {
                    let index = qi % vk.codeword_len();
                    let alphabet = luts.lookup_and_sum(interleaved_alphabet);
                    alphabet == codeword[index]
                },
            )
        })
}
