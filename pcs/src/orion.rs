//! Orion polynomial commitment scheme prototype implementaiton.
//! Includes implementation for Orion Expander-Code.

use std::{cmp, iter, marker::PhantomData, ops::Mul};

use arith::{Field, FieldSerde, FieldSerdeError, SimdField};
use ark_std::{iterable::Iterable, log2};
use polynomials::{EqPolynomial, MultiLinearPoly};
use rand::seq::index;
use thiserror::Error;
use transcript::Transcript;

use crate::PolynomialCommitmentScheme;

/******************************
 * PCS ERROR AND RESULT SETUP *
 ******************************/

#[derive(Debug, Error)]
pub enum OrionPCSError {
    #[error("Orion PCS linear code parameter unmatch error")]
    ParameterUnmatchError,

    #[error("field serde error")]
    SerializationError(#[from] FieldSerdeError),
}

pub type OrionResult<T> = std::result::Result<T, OrionPCSError>;

/********************************************
 * IMPLEMENTATIONS FOR ORION EXPANDER GRAPH *
 ********************************************/

type DiredtedEdge = usize;

type DirectedNeighboring = Vec<DiredtedEdge>;

#[derive(Clone, Debug)]
pub struct OrionExpanderGraph {
    // L R vertices size book keeping:
    // keep track of message length (l), and "compressed" code length (r)
    pub l_vertices_size: usize,
    pub r_vertices_size: usize,

    // neighboring stands for all (weighted) connected vertices of a vertex.
    // In this context, the neighborings stands for the neighborings
    // of vertices in R set of the bipariate graph, which explains why it has
    // size of l_vertices_size, while each neighboring reserved r_vertices_size
    // capacity.
    pub neighborings: Vec<DirectedNeighboring>,
}

impl OrionExpanderGraph {
    pub fn new(
        l_vertices_size: usize,
        r_vertices_size: usize,
        expanding_degree: usize,
        mut rng: impl rand::RngCore,
    ) -> Self {
        let mut neighborings: Vec<DirectedNeighboring> =
            vec![Vec::with_capacity(l_vertices_size); r_vertices_size];

        (0..l_vertices_size).for_each(|l_index| {
            let random_r_vertices = index::sample(&mut rng, r_vertices_size, expanding_degree);

            random_r_vertices
                .iter()
                .for_each(|r_index| neighborings[r_index].push(l_index))
        });

        Self {
            neighborings,
            l_vertices_size,
            r_vertices_size,
        }
    }

    #[inline(always)]
    pub fn expander_mul<F: Field>(
        &self,
        l_vertices: &[F],
        r_vertices: &mut [F],
    ) -> OrionResult<()> {
        if l_vertices.len() != self.l_vertices_size || r_vertices.len() != self.r_vertices_size {
            return Err(OrionPCSError::ParameterUnmatchError);
        }

        r_vertices
            .iter_mut()
            .zip(self.neighborings.iter())
            .for_each(|(ri, ni)| {
                *ri = ni.iter().map(|&edge_i| l_vertices[edge_i]).sum();
            });

        Ok(())
    }
}

/******************************************************
 * IMPLEMENTATIONS FOR ORION CODE FROM EXPANDER GRAPH *
 ******************************************************/

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrionCodeParameter {
    // parameter for graph g0, that maps n -> (\alpha_g0 n)
    // \alpha_g0 should be ranging in (0, 1)
    pub alpha_g0: f64,
    pub degree_g0: usize,

    // parameter regarding graph generation for the code:
    // stopping condition when message is too short for the recursive code
    // in the next round.
    pub length_threshold_g0s: usize,

    // parameter for graph g1, let the message in the middle has length L,
    // then the graph g1 maps L -> (\alpha_g1 L)
    pub alpha_g1: f64,
    pub degree_g1: usize,

    // code's relateive distance
    pub hamming_weight: f64,
}

// NOTE: This instance of code derives from Orion paper Section 5.
pub const ORION_CODE_PARAMETER_INSTANCE: OrionCodeParameter = OrionCodeParameter {
    alpha_g0: 0.33,
    degree_g0: 6,

    length_threshold_g0s: 12,

    alpha_g1: 0.337,
    degree_g1: 6,

    hamming_weight: 0.055,
};

// ACKNOWLEDGEMENT: on alphabet being F2 binary case, we appreciate the help from
// - Section 18 in essential coding theory
//   https://cse.buffalo.edu/faculty/atri/courses/coding-theory/book/web-coding-book.pdf
// - Notes from coding theory
//   https://www.cs.cmu.edu/~venkatg/teaching/codingtheory/notes/notes8.pdf
// - Druk-Ishai 2014
//   https://dl.acm.org/doi/10.1145/2554797.2554815

#[derive(Clone, Debug)]
pub struct OrionExpanderGraphPositioned {
    pub graph: OrionExpanderGraph,

    pub input_starts: usize,
    pub output_starts: usize,
    pub output_ends: usize,
}

impl OrionExpanderGraphPositioned {
    #[inline(always)]
    pub fn new(
        input_starts: usize,
        output_starts: usize,
        output_ends: usize,
        expanding_degree: usize,
        mut rng: impl rand::RngCore,
    ) -> Self {
        Self {
            graph: OrionExpanderGraph::new(
                output_starts - input_starts,
                output_ends - output_starts + 1,
                expanding_degree,
                &mut rng,
            ),
            input_starts,
            output_starts,
            output_ends,
        }
    }

    #[inline(always)]
    pub fn expander_mul<F: Field>(&self, buffer: &mut [F], scratch: &mut [F]) -> OrionResult<()> {
        let input_ref = &buffer[self.input_starts..self.output_starts];
        let output_ref = &mut scratch[self.output_starts..self.output_ends + 1];

        self.graph.expander_mul(input_ref, output_ref)?;
        buffer[self.output_starts..self.output_ends + 1].copy_from_slice(output_ref);

        Ok(())
    }
}

// NOTE: The OrionCode here is representing an instance of Spielman code
// (Spielman96), that relies on 2 lists of expander graphs serving as
// error reduction code, and thus the linear error correction code derive
// from the parity matrices corresponding to these expander graphs.
#[derive(Clone, Debug)]
pub struct OrionCode {
    pub params: OrionCodeParameter,

    // empirical parameters for this instance of expander code on input/codeword
    pub msg_len: usize,
    pub codeword_len: usize,

    // g0s (affecting left side alphabets of the codeword)
    // generated from the largest to the smallest
    pub g0s: Vec<OrionExpanderGraphPositioned>,

    // g1s (affecting right side alphabets of the codeword)
    // generated from the smallest to the largest
    pub g1s: Vec<OrionExpanderGraphPositioned>,
}

pub type OrionCodeword<F> = Vec<F>;

impl OrionCode {
    pub fn new(params: OrionCodeParameter, msg_len: usize, mut rng: impl rand::RngCore) -> Self {
        // NOTE: sanity check - 1 / threshold_len > hamming_weight
        // as was part of Druk-Ishai-14 distance proof by induction
        assert!(1f64 / (params.length_threshold_g0s as f64) > params.hamming_weight);

        // NOTE: sanity check for both alpha_g0 and alpha_g1
        assert!(0f64 < params.alpha_g0 && params.alpha_g0 < 1f64);
        assert!(0f64 < params.alpha_g1 && params.alpha_g1 < 1f64);

        // NOTE: the real deal of code instance generation starts here
        let mut recursive_g0_output_starts: Vec<usize> = Vec::new();

        let mut g0s: Vec<OrionExpanderGraphPositioned> = Vec::new();
        let mut g1s: Vec<OrionExpanderGraphPositioned> = Vec::new();

        let mut g0_input_starts = 0;
        let mut g0_output_starts = msg_len;

        while g0_output_starts - g0_input_starts > params.length_threshold_g0s {
            let n = g0_output_starts - g0_input_starts;
            let g0_output_len = (n as f64 * params.alpha_g0).round() as usize;
            let degree_g0 = cmp::min(params.degree_g0, g0_output_len);

            g0s.push(OrionExpanderGraphPositioned::new(
                g0_input_starts,
                g0_output_starts,
                g0_output_starts + g0_output_len - 1,
                degree_g0,
                &mut rng,
            ));

            recursive_g0_output_starts.push(g0_output_starts);

            (g0_input_starts, g0_output_starts) =
                (g0_output_starts, g0_output_starts + g0_output_len);
        }

        // After g0s are generated, we generate g1s
        let mut g1_output_starts = g0_output_starts;

        while let Some(g1_input_starts) = recursive_g0_output_starts.pop() {
            let n = g1_output_starts - g1_input_starts;
            let g1_output_len = (n as f64 * params.alpha_g1).round() as usize;
            let degree_g1 = cmp::min(params.degree_g1, g1_output_len);

            g1s.push(OrionExpanderGraphPositioned::new(
                g1_input_starts,
                g1_output_starts,
                g1_output_starts + g1_output_len - 1,
                degree_g1,
                &mut rng,
            ));

            g1_output_starts += g1_output_len;
        }

        let codeword_len = g1_output_starts;
        Self {
            params,
            msg_len,
            codeword_len,
            g0s,
            g1s,
        }
    }

    #[inline(always)]
    pub fn code_len(&self) -> usize {
        self.codeword_len
    }

    #[inline(always)]
    pub fn msg_len(&self) -> usize {
        self.msg_len
    }

    #[inline(always)]
    pub fn hamming_weight(&self) -> f64 {
        self.params.hamming_weight
    }

    #[inline(always)]
    pub fn encode<F: Field>(&self, msg: &[F]) -> OrionResult<OrionCodeword<F>> {
        let mut codeword = vec![F::ZERO; self.code_len()];
        self.encode_in_place(msg, &mut codeword)?;
        Ok(codeword)
    }

    #[inline(always)]
    pub fn encode_in_place<F: Field>(&self, msg: &[F], buffer: &mut [F]) -> OrionResult<()> {
        if msg.len() != self.msg_len() || buffer.len() != self.code_len() {
            return Err(OrionPCSError::ParameterUnmatchError);
        }

        buffer[..self.msg_len()].copy_from_slice(msg);
        let mut scratch = vec![F::ZERO; self.code_len()];

        self.g0s
            .iter()
            .chain(self.g1s.iter())
            .try_for_each(|g| g.expander_mul(buffer, &mut scratch))
    }
}

/****************************************
 * IMPLEMENTATIONS FOR MATRIX TRANSPOSE *
 ****************************************/

pub(crate) const fn cache_batch_size<F: Sized>() -> usize {
    const CACHE_SIZE: usize = 1 << 16;
    CACHE_SIZE / size_of::<F>()
}

#[inline(always)]
pub(crate) fn transpose_in_place<F: Field>(mat: &mut [F], scratch: &mut [F], row_num: usize) {
    let col_num = mat.len() / row_num;
    let batch_size = cache_batch_size::<F>();

    mat.chunks(batch_size)
        .enumerate()
        .for_each(|(i, ith_batch)| {
            let batch_srt = batch_size * i;

            ith_batch.iter().enumerate().for_each(|(j, &elem_j)| {
                let src = batch_srt + j;
                let dst = (src / col_num) + (src % col_num) * row_num;

                scratch[dst] = elem_j;
            })
        });

    mat.copy_from_slice(scratch);
}

#[inline]
pub(crate) fn simd_inner_prod<F0, F1, IPPackF0, IPPackF1>(
    l: &[F0],
    r: &[F1],
    scratch_pl: &mut [IPPackF0],
    scratch_pr: &mut [IPPackF1],
) -> F1
where
    F0: Field,
    F1: Field + From<F0> + Mul<F0, Output = F1>,
    IPPackF0: SimdField<Scalar = F0>,
    IPPackF1: SimdField<Scalar = F1> + Mul<IPPackF0, Output = IPPackF1>,
{
    assert_eq!(l.len() % IPPackF0::PACK_SIZE, 0);
    assert_eq!(r.len() % IPPackF1::PACK_SIZE, 0);

    scratch_pl
        .iter_mut()
        .zip(l.chunks(IPPackF0::PACK_SIZE))
        .for_each(|(pl, ls)| *pl = IPPackF0::pack(ls));

    scratch_pr
        .iter_mut()
        .zip(r.chunks(IPPackF1::PACK_SIZE))
        .for_each(|(pr, rs)| *pr = IPPackF1::pack(rs));

    let simd_sum: IPPackF1 = scratch_pl
        .iter()
        .zip(scratch_pr.iter())
        .map(|(pl, pr)| *pr * *pl)
        .sum();

    simd_sum.unpack().iter().sum()
}

/**********************************************************
 * IMPLEMENTATIONS FOR ORION POLYNOMIAL COMMITMENT SCHEME *
 **********************************************************/

pub const ORION_PCS_SOUNDNESS_BITS: usize = 128;

#[derive(Clone, Debug)]
pub struct OrionPublicParams {
    pub num_variables: usize,
    pub code_instance: OrionCode,
}

#[derive(Clone, Debug)]
pub struct OrionCommitmentWithData<F, PackF>
where
    F: Field + FieldSerde,
    PackF: SimdField<Scalar = F>,
{
    pub interleaved_alphabet_tree: tree::Tree,

    pub _phantom: PhantomData<PackF>,
}

pub type OrionCommitment = tree::Node;

impl<F, PackF> From<OrionCommitmentWithData<F, PackF>> for OrionCommitment
where
    F: Field + FieldSerde,
    PackF: SimdField<Scalar = F>,
{
    fn from(value: OrionCommitmentWithData<F, PackF>) -> Self {
        value.interleaved_alphabet_tree.root()
    }
}

type OrionProximityCodeword<F> = Vec<F>;

#[derive(Clone, Debug)]
pub struct OrionProof<EvalF: Field + FieldSerde> {
    pub eval_row: Vec<EvalF>,
    pub proximity_rows: Vec<OrionProximityCodeword<EvalF>>,

    pub query_openings: Vec<tree::RangePath>,
}

impl OrionPublicParams {
    #[inline(always)]
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
        let code_len_over_f_bits = field_size_bits - log2(self.code_instance.code_len()) as usize;

        (soundness_bits as f64 / code_len_over_f_bits as f64).ceil() as usize
    }

    pub fn commit<F, PackF>(
        &self,
        poly: &MultiLinearPoly<F>,
    ) -> OrionResult<OrionCommitmentWithData<F, PackF>>
    where
        F: Field + FieldSerde,
        PackF: SimdField<Scalar = F>,
    {
        let (row_num, msg_size) = Self::row_col_from_variables::<F>(poly.get_num_vars());

        // NOTE: pre transpose evaluations
        let mut transposed_evaluations = poly.coeffs.clone();
        let mut scratch = vec![F::ZERO; 1 << poly.get_num_vars()];
        transpose_in_place(&mut transposed_evaluations, &mut scratch, row_num);
        drop(scratch);

        // NOTE: SIMD pack each row of transposed matrix
        let mut packed_evals: Vec<PackF> = transposed_evaluations
            .chunks(PackF::PACK_SIZE)
            .map(SimdField::pack)
            .collect();
        drop(transposed_evaluations);

        // NOTE: transpose back to rows of evaluations, but packed
        let packed_rows = row_num / PackF::PACK_SIZE;

        let mut scratch = vec![PackF::ZERO; packed_rows * msg_size];
        transpose_in_place(&mut packed_evals, &mut scratch, msg_size);
        drop(scratch);

        // NOTE: packed codeword buffer and encode over packed field
        let mut packed_interleaved_codewords = vec![PackF::ZERO; packed_rows * self.code_len()];
        packed_evals
            .chunks(msg_size)
            .zip(packed_interleaved_codewords.chunks_mut(self.code_len()))
            .try_for_each(|(evals, codeword)| {
                self.code_instance.encode_in_place(evals, codeword)
            })?;
        drop(packed_evals);

        // NOTE: transpose codeword s.t., the matrix has codewords being columns
        let mut scratch = vec![PackF::ZERO; packed_rows * self.code_len()];
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
        let interleaved_alphabet_tree = tree::Tree::compact_new_with_packed_field_elems::<F, PackF>(
            &packed_interleaved_codewords,
        );

        Ok(OrionCommitmentWithData {
            interleaved_alphabet_tree,

            _phantom: PhantomData,
        })
    }

    pub fn open<F, PackF, EvalF, IPPackF, IPPackEvalF, T>(
        &self,
        poly: &MultiLinearPoly<F>,
        commitment_with_data: &OrionCommitmentWithData<F, PackF>,
        point: &[EvalF],
        transcript: &mut T,
    ) -> (EvalF, OrionProof<EvalF>)
    where
        F: Field + FieldSerde,
        PackF: SimdField<Scalar = F>,
        EvalF: Field + FieldSerde + From<F> + Mul<F, Output = EvalF>,
        IPPackF: SimdField<Scalar = F>,
        IPPackEvalF: SimdField<Scalar = EvalF> + Mul<IPPackF, Output = IPPackEvalF>,
        T: Transcript<EvalF>,
    {
        assert_eq!(IPPackEvalF::PACK_SIZE, IPPackF::PACK_SIZE);

        let (row_num, msg_size) = Self::row_col_from_variables::<F>(poly.get_num_vars());
        let num_of_vars_in_codeword = log2(msg_size) as usize;

        // NOTE: transpose evaluations for linear combinations in evaulation/proximity tests
        let mut transposed_evaluations = poly.coeffs.clone();
        let mut scratch = vec![F::ZERO; 1 << poly.get_num_vars()];
        transpose_in_place(&mut transposed_evaluations, &mut scratch, row_num);
        drop(scratch);

        // NOTE: prepare scratch space for both evals and proximity test
        let mut scratch_pf = vec![IPPackF::ZERO; row_num / IPPackF::PACK_SIZE];
        let mut scratch_pef = vec![IPPackEvalF::ZERO; row_num / IPPackEvalF::PACK_SIZE];

        // NOTE: working on evaluation response of tensor code IOP based PCS
        let eq_linear_comb = EqPolynomial::build_eq_x_r(&point[num_of_vars_in_codeword..]);
        let mut eval_row = vec![EvalF::ZERO; msg_size];
        transposed_evaluations
            .chunks(row_num)
            .zip(eval_row.iter_mut())
            .for_each(|(col_i, res_i)| {
                *res_i = simd_inner_prod(col_i, &eq_linear_comb, &mut scratch_pf, &mut scratch_pef);
            });

        // NOTE: working on evaluation on top of evaluation response
        let eq_linear_comb = EqPolynomial::build_eq_x_r(&point[..num_of_vars_in_codeword]);
        let mut scratch_msg_sized_0 = vec![IPPackEvalF::ZERO; msg_size / IPPackEvalF::PACK_SIZE];
        let mut scratch_msg_sized_1 = vec![IPPackEvalF::ZERO; msg_size / IPPackEvalF::PACK_SIZE];
        let eval = simd_inner_prod(
            &eval_row,
            &eq_linear_comb,
            &mut scratch_msg_sized_0,
            &mut scratch_msg_sized_1,
        );
        drop(scratch_msg_sized_0);
        drop(scratch_msg_sized_1);

        // NOTE: draw random linear combination out
        // and compose proximity response(s) of tensor code IOP based PCS
        let proximity_repetitions =
            self.proximity_repetition_num(ORION_PCS_SOUNDNESS_BITS, EvalF::FIELD_SIZE);
        let mut proximity_rows = vec![vec![EvalF::ZERO; msg_size]; proximity_repetitions];

        (0..proximity_repetitions).for_each(|rep_i| {
            let random_coeffs = transcript.generate_challenge_field_elements(row_num);

            transposed_evaluations
                .chunks(row_num)
                .zip(proximity_rows[rep_i].iter_mut())
                .for_each(|(col_i, res_i)| {
                    *res_i =
                        simd_inner_prod(col_i, &random_coeffs, &mut scratch_pf, &mut scratch_pef);
                });
        });

        // NOTE: scratch space for evals and proximity test life cycle finish
        drop(scratch_pf);
        drop(scratch_pef);

        // NOTE: MT opening for point queries
        let leaf_range = row_num * F::FIELD_SIZE / (tree::LEAF_BYTES * 8);
        let query_num = self.query_complexity(ORION_PCS_SOUNDNESS_BITS);
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

    pub fn verify<F, PackF, EvalF, IPPackF, IPPackEvalF, T>(
        &self,
        commitment: &OrionCommitment,
        point: &[EvalF],
        evaluation: EvalF,
        proof: &OrionProof<EvalF>,
        transcript: &mut T,
    ) -> bool
    where
        F: Field + FieldSerde,
        PackF: SimdField<Scalar = F>,
        EvalF: Field + FieldSerde + From<F> + Mul<F, Output = EvalF>,
        IPPackF: SimdField<Scalar = F>,
        IPPackEvalF: SimdField<Scalar = EvalF> + Mul<IPPackF, Output = IPPackEvalF>,
        T: Transcript<EvalF>,
    {
        let (row_num, msg_size) = Self::row_col_from_variables::<F>(point.len());
        let num_of_vars_in_codeword = log2(msg_size) as usize;

        // NOTE: working on evaluation response, evaluate the rest of the response
        let eq_x_r = EqPolynomial::build_eq_x_r(&point[..num_of_vars_in_codeword]);
        let mut scratch_msg_sized_0 = vec![IPPackEvalF::ZERO; msg_size / IPPackEvalF::PACK_SIZE];
        let mut scratch_msg_sized_1 = vec![IPPackEvalF::ZERO; msg_size / IPPackEvalF::PACK_SIZE];
        let final_eval = simd_inner_prod(
            &proof.eval_row,
            &eq_x_r,
            &mut scratch_msg_sized_0,
            &mut scratch_msg_sized_1,
        );
        drop(scratch_msg_sized_0);
        drop(scratch_msg_sized_1);
        if final_eval != evaluation {
            return false;
        }

        // NOTE: working on proximity responses, draw random linear combinations
        // then draw query points from fiat shamir transcripts
        let proximity_test_num =
            self.proximity_repetition_num(ORION_PCS_SOUNDNESS_BITS, EvalF::FIELD_SIZE);
        let random_linear_combinations: Vec<Vec<EvalF>> = (0..proximity_test_num)
            .map(|_| transcript.generate_challenge_field_elements(row_num))
            .collect();
        let query_num = self.query_complexity(ORION_PCS_SOUNDNESS_BITS);
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

        // NOTE: encode the proximity/evaluation responses,
        // check againts all challenged indices by check alphabets against
        // linear combined interleaved alphabet
        let mut scratch_pf = vec![IPPackF::ZERO; row_num / IPPackF::PACK_SIZE];
        let mut scratch_pef = vec![IPPackEvalF::ZERO; row_num / IPPackEvalF::PACK_SIZE];

        let eq_linear_combination = EqPolynomial::build_eq_x_r(&point[num_of_vars_in_codeword..]);
        random_linear_combinations
            .iter()
            .zip(proof.proximity_rows.iter())
            .chain(iter::once((&eq_linear_combination, &proof.eval_row)))
            .all(|(rl, msg)| match self.code_instance.encode(msg) {
                Ok(codeword) => {
                    query_points
                        .iter()
                        .zip(proof.query_openings.iter())
                        .all(|(&qi, range_path)| {
                            let interleaved_alphabet = range_path.unpack_field_elems::<F, PackF>();
                            let alphabet = simd_inner_prod(
                                &interleaved_alphabet,
                                rl,
                                &mut scratch_pf,
                                &mut scratch_pef,
                            );
                            alphabet == codeword[qi]
                        })
                }
                _ => false,
            })
    }
}

/***************************************************
 * POLYNOMIAL COMMITMENT TRAIT ALIGNMENT FOR ORION *
 ***************************************************/

pub struct OrionPCS<F, PackF, EvalF, IPPackF, IPPackEvalF, T>
where
    F: Field + FieldSerde,
    PackF: SimdField<Scalar = F>,
    EvalF: Field + FieldSerde + From<F> + Mul<F, Output = EvalF>,
    IPPackF: SimdField<Scalar = F>,
    IPPackEvalF: SimdField<Scalar = EvalF> + Mul<IPPackF, Output = IPPackEvalF>,
    T: Transcript<EvalF>,
{
    _marker_f: PhantomData<F>,
    _marker_pack_f: PhantomData<PackF>,
    _marker_eval_f: PhantomData<EvalF>,
    _marker_pack_f0: PhantomData<IPPackF>,
    _marker_pack_eval_f: PhantomData<IPPackEvalF>,
    _marker_t: PhantomData<T>,
}

#[derive(Clone, Debug)]
pub struct OrionPCSSetup {
    pub num_vars: usize,
    pub code_parameter: OrionCodeParameter,
}

impl<F, PackF, EvalF, IPPackF, IPPackEvalF, T> PolynomialCommitmentScheme
    for OrionPCS<F, PackF, EvalF, IPPackF, IPPackEvalF, T>
where
    F: Field + FieldSerde,
    PackF: SimdField<Scalar = F>,
    EvalF: Field + FieldSerde + From<F> + Mul<F, Output = EvalF>,
    IPPackF: SimdField<Scalar = F>,
    IPPackEvalF: SimdField<Scalar = EvalF> + Mul<IPPackF, Output = IPPackEvalF>,
    T: Transcript<EvalF>,
{
    type PublicParams = OrionPCSSetup;

    type Poly = MultiLinearPoly<F>;

    type EvalPoint = Vec<EvalF>;
    type Eval = EvalF;

    type SRS = OrionPublicParams;
    type ProverKey = Self::SRS;
    type VerifierKey = Self::SRS;

    type Commitment = OrionCommitment;
    type CommitmentWithData = OrionCommitmentWithData<F, PackF>;
    type OpeningProof = OrionProof<EvalF>;

    type FiatShamirTranscript = T;

    fn gen_srs_for_testing(rng: impl rand::RngCore, params: &Self::PublicParams) -> Self::SRS {
        OrionPublicParams::from_random::<F>(params.num_vars, params.code_parameter, rng)
    }

    fn commit(proving_key: &Self::ProverKey, poly: &Self::Poly) -> Self::CommitmentWithData {
        proving_key.commit(poly).unwrap()
    }

    fn open(
        proving_key: &Self::ProverKey,
        poly: &Self::Poly,
        opening_point: &Self::EvalPoint,
        commitment_with_data: &Self::CommitmentWithData,
        transcript: &mut Self::FiatShamirTranscript,
    ) -> (Self::Eval, Self::OpeningProof) {
        proving_key.open::<F, PackF, EvalF, IPPackF, IPPackEvalF, T>(
            poly,
            commitment_with_data,
            opening_point,
            transcript,
        )
    }

    fn verify(
        verifying_key: &Self::VerifierKey,
        commitment: &Self::Commitment,
        opening_point: &Self::EvalPoint,
        evaluation: Self::Eval,
        opening_proof: &Self::OpeningProof,
        transcript: &mut Self::FiatShamirTranscript,
    ) -> bool {
        verifying_key.verify::<F, PackF, EvalF, IPPackF, IPPackEvalF, T>(
            commitment,
            opening_point,
            evaluation,
            opening_proof,
            transcript,
        )
    }
}
