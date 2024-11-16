use std::{iter, marker::PhantomData, ops::Mul};

use arith::{Field, FieldSerde, SimdField};
use polynomials::{EqPolynomial, MultiLinearPoly};
use transcript::Transcript;

use crate::{orion::utils::LookupTables, PCS_SOUNDNESS_BITS};

use super::{
    linear_code::{OrionCode, OrionCodeParameter},
    utils::{transpose_in_place, OrionPCSError, OrionResult},
};

/**********************************************************
 * IMPLEMENTATIONS FOR ORION POLYNOMIAL COMMITMENT SCHEME *
 **********************************************************/

#[derive(Clone, Debug)]
pub struct OrionPublicParams {
    pub num_variables: usize,
    pub code_instance: OrionCode,
}

#[derive(Clone, Debug)]
pub struct OrionCommitmentWithData<F, ComPackF>
where
    F: Field + FieldSerde,
    ComPackF: SimdField<Scalar = F>,
{
    pub interleaved_alphabet_tree: tree::Tree,

    pub _phantom: PhantomData<ComPackF>,
}

pub type OrionCommitment = tree::Node;

impl<F, ComPackF> From<OrionCommitmentWithData<F, ComPackF>> for OrionCommitment
where
    F: Field + FieldSerde,
    ComPackF: SimdField<Scalar = F>,
{
    fn from(value: OrionCommitmentWithData<F, ComPackF>) -> Self {
        value.interleaved_alphabet_tree.root()
    }
}

#[derive(Clone, Debug)]
pub struct OrionProof<EvalF: Field + FieldSerde> {
    pub eval_row: Vec<EvalF>,
    pub proximity_rows: Vec<Vec<EvalF>>,

    pub query_openings: Vec<tree::RangePath>,
}

impl OrionPublicParams {
    pub(crate) fn row_col_from_variables<F: Field>(num_variables: usize) -> (usize, usize) {
        let poly_variables: usize = num_variables;

        let elems_for_smallest_tree = tree::leaf_adic::<F>() * 2;

        let row_num: usize = elems_for_smallest_tree;
        let msg_size: usize = (1 << poly_variables) / row_num;

        (row_num, msg_size)
    }

    pub fn new<F: Field>(num_variables: usize, code_instance: OrionCode) -> OrionResult<Self> {
        let (_, msg_size) = Self::row_col_from_variables::<F>(num_variables);
        if msg_size != code_instance.msg_len() {
            return Err(OrionPCSError::ParameterUnmatchError);
        }

        // NOTE: we just move the instance of code,
        // don't think the instance of expander code will be used elsewhere
        Ok(Self {
            num_variables,
            code_instance,
        })
    }

    pub fn from_random<F: Field>(
        num_variables: usize,
        code_param_instance: OrionCodeParameter,
        mut rng: impl rand::RngCore,
    ) -> Self {
        let (_, msg_size) = Self::row_col_from_variables::<F>(num_variables);

        Self {
            num_variables,
            code_instance: OrionCode::new(code_param_instance, msg_size, &mut rng),
        }
    }

    pub fn code_len(&self) -> usize {
        self.code_instance.code_len()
    }

    pub fn query_complexity(&self, soundness_bits: usize) -> usize {
        // NOTE: use Ligero (AHIV22) or Avg-case dist to a code (BKS18)
        // version of avg case dist in unique decoding technique.
        let avg_case_dist = self.code_instance.hamming_weight() / 3f64;
        let sec_bits = -(1f64 - avg_case_dist).log2();

        (soundness_bits as f64 / sec_bits).ceil() as usize
    }

    pub fn proximity_repetition_num(&self, soundness_bits: usize, field_size_bits: usize) -> usize {
        // NOTE: use Ligero (AHIV22) or Avg-case dist to a code (BKS18)
        // version of avg case dist in unique decoding technique.
        // Here is the probability union bound
        let code_len_over_f_bits = field_size_bits - self.code_instance.code_len().ilog2() as usize;

        (soundness_bits as f64 / code_len_over_f_bits as f64).ceil() as usize
    }

    pub fn commit<F, ComPackF>(
        &self,
        poly: &MultiLinearPoly<F>,
    ) -> OrionResult<OrionCommitmentWithData<F, ComPackF>>
    where
        F: Field + FieldSerde,
        ComPackF: SimdField<Scalar = F>,
    {
        let (row_num, msg_size) = Self::row_col_from_variables::<F>(poly.get_num_vars());

        // NOTE: pre transpose evaluations
        let mut transposed_evaluations = poly.coeffs.clone();
        let mut scratch = vec![F::ZERO; 1 << poly.get_num_vars()];
        transpose_in_place(&mut transposed_evaluations, &mut scratch, row_num);
        drop(scratch);

        // NOTE: SIMD pack each row of transposed matrix
        assert_eq!(transposed_evaluations.len() % ComPackF::PACK_SIZE, 0);
        let mut packed_evals: Vec<ComPackF> = transposed_evaluations
            .chunks(ComPackF::PACK_SIZE)
            .map(SimdField::pack)
            .collect();
        drop(transposed_evaluations);

        // NOTE: transpose back to rows of evaluations, but packed
        let packed_rows = row_num / ComPackF::PACK_SIZE;
        assert_eq!(row_num % ComPackF::PACK_SIZE, 0);

        let mut scratch = vec![ComPackF::ZERO; packed_rows * msg_size];
        transpose_in_place(&mut packed_evals, &mut scratch, msg_size);
        drop(scratch);

        // NOTE: packed codeword buffer and encode over packed field
        let mut packed_interleaved_codewords = vec![ComPackF::ZERO; packed_rows * self.code_len()];
        packed_evals
            .chunks(msg_size)
            .zip(packed_interleaved_codewords.chunks_mut(self.code_len()))
            .try_for_each(|(evals, codeword)| {
                self.code_instance.encode_in_place(evals, codeword)
            })?;
        drop(packed_evals);

        // NOTE: transpose codeword s.t., the matrix has codewords being columns
        let mut scratch = vec![ComPackF::ZERO; packed_rows * self.code_len()];
        transpose_in_place(&mut packed_interleaved_codewords, &mut scratch, packed_rows);
        drop(scratch);

        // NOTE: commit the interleaved codeword
        // we just directly commit to the packed field elements to leaves
        // Also note, when codeword is not power of 2 length, pad to nearest po2
        // to commit by merkle tree
        if !packed_interleaved_codewords.len().is_power_of_two() {
            let aligned_po2_len = packed_interleaved_codewords.len().next_power_of_two();
            packed_interleaved_codewords.resize(aligned_po2_len, ComPackF::ZERO);
        }
        let mt = tree::Tree::compact_new_with_packed_field_elems::<F, ComPackF>(
            &packed_interleaved_codewords,
        );

        Ok(OrionCommitmentWithData {
            interleaved_alphabet_tree: mt,
            _phantom: PhantomData,
        })
    }

    pub fn open<F, EvalF, ComPackF, OpenPackF, T>(
        &self,
        poly: &MultiLinearPoly<F>,
        commitment_with_data: &OrionCommitmentWithData<F, ComPackF>,
        point: &[EvalF],
        transcript: &mut T,
    ) -> (EvalF, OrionProof<EvalF>)
    where
        F: Field + FieldSerde,
        EvalF: Field + FieldSerde + From<F> + Mul<F, Output = EvalF>,
        ComPackF: SimdField<Scalar = F>,
        OpenPackF: SimdField<Scalar = F>,
        T: Transcript<EvalF>,
    {
        let (row_num, msg_size) = Self::row_col_from_variables::<F>(poly.get_num_vars());
        let num_of_vars_in_codeword = msg_size.ilog2() as usize;

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
        let mut luts = LookupTables::new(OpenPackF::PACK_SIZE, row_num / OpenPackF::PACK_SIZE);

        // NOTE: working on evaluation response of tensor code IOP based PCS
        let eq_linear_comb = EqPolynomial::build_eq_x_r(&point[num_of_vars_in_codeword..]);
        luts.build(&eq_linear_comb);

        let mut eval_row = vec![EvalF::ZERO; msg_size];
        packed_evals
            .chunks(row_num / OpenPackF::PACK_SIZE)
            .zip(eval_row.iter_mut())
            .for_each(|(p_col, res)| *res = luts.lookup_and_sum(p_col));

        // NOTE: draw random linear combination out
        // and compose proximity response(s) of tensor code IOP based PCS
        let proximity_repetitions =
            self.proximity_repetition_num(PCS_SOUNDNESS_BITS, EvalF::FIELD_SIZE);
        let mut proximity_rows = vec![vec![EvalF::ZERO; msg_size]; proximity_repetitions];

        proximity_rows.iter_mut().for_each(|row_buffer| {
            let random_coeffs = transcript.generate_challenge_field_elements(row_num);

            luts.build(&random_coeffs);

            packed_evals
                .chunks(row_num / OpenPackF::PACK_SIZE)
                .zip(row_buffer.iter_mut())
                .for_each(|(p_col, res)| *res = luts.lookup_and_sum(p_col));
        });

        // NOTE: working on evaluation on top of evaluation response
        let eq_xr = EqPolynomial::build_eq_x_r(&point[..num_of_vars_in_codeword]);
        let eval = eval_row
            .iter()
            .zip(eq_xr.iter())
            .map(|(&e, &r)| e * r)
            .sum();

        // NOTE: MT opening for point queries
        let leaf_range = row_num * F::FIELD_SIZE / (tree::LEAF_BYTES * 8);
        let query_num = self.query_complexity(PCS_SOUNDNESS_BITS);
        let mut query_points = transcript.generate_challenge_index_vector(query_num);
        let query_openings = query_points
            .iter_mut()
            .map(|qi| {
                *qi %= self.code_len();
                let left = *qi * leaf_range;
                let right = left + leaf_range - 1;

                commitment_with_data
                    .interleaved_alphabet_tree
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

    pub fn verify<F, EvalF, ComPackF, OpenPackF, T>(
        &self,
        commitment: &OrionCommitment,
        point: &[EvalF],
        evaluation: EvalF,
        proof: &OrionProof<EvalF>,
        transcript: &mut T,
    ) -> bool
    where
        F: Field + FieldSerde,
        EvalF: Field + FieldSerde + From<F> + Mul<F, Output = EvalF>,
        ComPackF: SimdField<Scalar = F>,
        OpenPackF: SimdField<Scalar = F>,
        T: Transcript<EvalF>,
    {
        let (row_num, msg_size) = Self::row_col_from_variables::<F>(point.len());
        let num_of_vars_in_codeword = msg_size.ilog2() as usize;

        // NOTE: working on evaluation response, evaluate the rest of the response
        let eq_xr = EqPolynomial::build_eq_x_r(&point[..num_of_vars_in_codeword]);
        let final_eval: EvalF = proof
            .eval_row
            .iter()
            .zip(eq_xr.iter())
            .map(|(&e, &r)| e * r)
            .sum();
        if final_eval != evaluation {
            return false;
        }

        // NOTE: working on proximity responses, draw random linear combinations
        // then draw query points from fiat shamir transcripts
        let proximity_test_num =
            self.proximity_repetition_num(PCS_SOUNDNESS_BITS, EvalF::FIELD_SIZE);
        let random_linear_combinations: Vec<Vec<EvalF>> = (0..proximity_test_num)
            .map(|_| transcript.generate_challenge_field_elements(row_num))
            .collect();
        let query_num = self.query_complexity(PCS_SOUNDNESS_BITS);
        let mut query_points = transcript.generate_challenge_index_vector(query_num);
        query_points.iter_mut().for_each(|qi| {
            *qi %= self.code_len();
        });

        // NOTE: check consistency in MT in the opening trees and against the commitment tree
        let leaf_range = row_num * F::FIELD_SIZE / (tree::LEAF_BYTES * 8);
        let mt_consistency =
            query_points
                .iter()
                .zip(proof.query_openings.iter())
                .all(|(&qi, range_path)| {
                    range_path.verify(commitment) && qi == range_path.left / leaf_range
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
        let mut luts = LookupTables::new(OpenPackF::PACK_SIZE, row_num / OpenPackF::PACK_SIZE);
        assert_eq!(row_num % OpenPackF::PACK_SIZE, 0);

        let eq_linear_combination = EqPolynomial::build_eq_x_r(&point[num_of_vars_in_codeword..]);
        random_linear_combinations
            .iter()
            .zip(proof.proximity_rows.iter())
            .chain(iter::once((&eq_linear_combination, &proof.eval_row)))
            .all(|(rl, msg)| {
                let codeword = match self.code_instance.encode(msg) {
                    Ok(c) => c,
                    _ => return false,
                };

                luts.build(rl);

                query_points
                    .iter()
                    .zip(packed_interleaved_alphabets.iter())
                    .all(|(&qi, interleaved_alphabet)| {
                        let alphabet = luts.lookup_and_sum(interleaved_alphabet);
                        alphabet == codeword[qi]
                    })
            })
    }
}
