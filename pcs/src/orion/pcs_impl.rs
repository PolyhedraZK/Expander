use std::{iter, marker::PhantomData, ops::Mul};

use arith::{Field, FieldSerde, SimdField};
use polynomials::{EqPolynomial, MultiLinearPoly};
use transcript::Transcript;

use crate::{traits::TensorCodeIOPPCS, PCS_SOUNDNESS_BITS};

use super::{
    linear_code::{OrionCode, OrionCodeParameter},
    utils::{transpose_in_place, OrionPCSError, OrionResult, SubsetSumLUTs},
};

/**********************************************************
 * IMPLEMENTATIONS FOR ORION POLYNOMIAL COMMITMENT SCHEME *
 **********************************************************/

#[derive(Clone, Debug)]
pub struct OrionPublicParams {
    pub num_variables: usize,
    pub code_instance: OrionCode,
}

impl TensorCodeIOPPCS for OrionPublicParams {
    fn codeword_len(&self) -> usize {
        self.code_instance.code_len()
    }

    fn hamming_weight(&self) -> f64 {
        self.code_instance.hamming_weight()
    }
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
    pub fn new<F: Field>(num_variables: usize, code_instance: OrionCode) -> OrionResult<Self> {
        let (_, msg_size) = Self::evals_shape::<F>(num_variables);
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
        let (_, msg_size) = Self::evals_shape::<F>(num_variables);

        Self {
            num_variables,
            code_instance: OrionCode::new(code_param_instance, msg_size, &mut rng),
        }
    }

    pub fn commit<F, ComPackF>(
        &self,
        poly: &MultiLinearPoly<F>,
    ) -> OrionResult<OrionCommitmentWithData<F, ComPackF>>
    where
        F: Field + FieldSerde,
        ComPackF: SimdField<Scalar = F>,
    {
        let (row_num, msg_size) = Self::evals_shape::<F>(poly.get_num_vars());

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
        let mut packed_interleaved_codewords =
            vec![ComPackF::ZERO; packed_rows * self.codeword_len()];
        packed_evals
            .chunks(msg_size)
            .zip(packed_interleaved_codewords.chunks_mut(self.codeword_len()))
            .try_for_each(|(evals, codeword)| {
                self.code_instance.encode_in_place(evals, codeword)
            })?;
        drop(packed_evals);

        // NOTE: transpose codeword s.t., the matrix has codewords being columns
        let mut scratch = vec![ComPackF::ZERO; packed_rows * self.codeword_len()];
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
        let (row_num, msg_size) = Self::evals_shape::<F>(poly.get_num_vars());
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
        let proximity_test_num = self.proximity_repetitions::<EvalF>(PCS_SOUNDNESS_BITS);
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
        let query_num = self.query_complexity(PCS_SOUNDNESS_BITS);
        let query_indices = transcript.generate_challenge_index_vector(query_num);
        let query_openings = query_indices
            .iter()
            .map(|qi| {
                let index = *qi % self.codeword_len();
                let left = index * leaf_range;
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
        let (row_num, msg_size) = Self::evals_shape::<F>(point.len());
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
        let proximity_test_num = self.proximity_repetitions::<EvalF>(PCS_SOUNDNESS_BITS);
        let random_linear_combinations: Vec<Vec<EvalF>> = (0..proximity_test_num)
            .map(|_| transcript.generate_challenge_field_elements(row_num))
            .collect();
        let query_num = self.query_complexity(PCS_SOUNDNESS_BITS);
        let query_indices = transcript.generate_challenge_index_vector(query_num);

        // NOTE: check consistency in MT in the opening trees and against the commitment tree
        let leaf_range = row_num / tree::leaf_adic::<F>();
        let mt_consistency =
            query_indices
                .iter()
                .zip(proof.query_openings.iter())
                .all(|(&qi, range_path)| {
                    let index = qi % self.codeword_len();
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
                let codeword = match self.code_instance.encode(msg) {
                    Ok(c) => c,
                    _ => return false,
                };

                luts.build(rl);

                query_indices
                    .iter()
                    .zip(packed_interleaved_alphabets.iter())
                    .all(|(&qi, interleaved_alphabet)| {
                        let index = qi % self.codeword_len();
                        let alphabet = luts.lookup_and_sum(interleaved_alphabet);
                        alphabet == codeword[index]
                    })
            })
    }
}
