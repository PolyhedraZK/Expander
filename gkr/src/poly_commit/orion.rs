//! Orion polynomial commitment scheme prototype implementaiton.
//! Includes implementation for Orion Expander-Code.

use std::{cmp, iter::once, marker::PhantomData};

use arith::{ExtensionField, Field, FieldSerde, FieldSerdeError, FieldSerdeResult, SimdField};
use ark_std::log2;
use polynomials::{EqPolynomial, MultiLinearPoly};
use rand::seq::index;
use thiserror::Error;
use transcript::Transcript;
use tree::{Leaf, Tree, LEAF_BYTES, LEAF_HASH_BYTES};

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

#[derive(Clone)]
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
    // empirical parameters for the expander code on input/output size
    // NOTE: the derived code rate and invert code rate should preserve
    // in the recursive code of smaller size that comes in later rounds
    pub input_message_len: usize,
    pub output_code_len: usize,

    // parameter for graph g0, that maps n -> (\alpha n)
    // alpha should be ranging in (0, 1)
    pub alpha_g0: f64,
    pub degree_g0: usize,

    // parameter regarding graph generation for the code:
    // stopping condition when message is too short for the recursive code
    // in the next round.
    pub lenghth_threshold_g0s: usize,

    // parameter for graph g1, let the message in the middle has length L,
    // then the graph g1 maps L -> ((rate_inv - 1) x n) - L
    pub degree_g1: usize,

    // TODO: code distances
    pub hamming_weight: f64,
}

impl OrionCodeParameter {
    #[inline(always)]
    pub fn code_rate(&self) -> f64 {
        self.input_message_len as f64 / self.output_code_len as f64
    }

    #[inline(always)]
    pub fn inv_code_rate(&self) -> f64 {
        self.output_code_len as f64 / self.input_message_len as f64
    }

    #[inline(always)]
    pub fn hamming_weight(&self) -> f64 {
        self.hamming_weight
    }
}

// ACKNOWLEDGEMENT: on alphabet being F2 binary case, we appreciate the help from
// - Section 18 in essential coding theory
//   https://cse.buffalo.edu/faculty/atri/courses/coding-theory/book/web-coding-book.pdf
// - Notes from coding theory
//   https://www.cs.cmu.edu/~venkatg/teaching/codingtheory/notes/notes8.pdf
// - Druk-Ishai 2014
//   https://dl.acm.org/doi/10.1145/2554797.2554815

// TODO: fix a set of Orion code parameters for message length ranging 2^5 - 2^15
// together with the code distances, where we need for query number computation

#[derive(Clone)]
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
#[derive(Clone)]
pub struct OrionCode {
    pub params: OrionCodeParameter,

    // g0s (affecting left side alphabets of the codeword)
    // generated from the largest to the smallest
    pub g0s: Vec<OrionExpanderGraphPositioned>,

    // g1s (affecting right side alphabets of the codeword)
    // generated from the smallest to the largest
    pub g1s: Vec<OrionExpanderGraphPositioned>,
}

pub type OrionCodeword<F> = Vec<F>;

impl OrionCode {
    pub fn new(params: OrionCodeParameter, mut rng: impl rand::RngCore) -> Self {
        let mut recursive_code_msg_code_starts: Vec<(usize, usize)> = Vec::new();

        let mut g0s: Vec<OrionExpanderGraphPositioned> = Vec::new();
        let mut g1s: Vec<OrionExpanderGraphPositioned> = Vec::new();

        let mut g0_input_starts = 0;
        let mut g0_output_starts = params.input_message_len;

        // For Spielman code, we keep recurse down til a point to stop,
        // and either the next subcodeword is too short for threshold,
        // or the next codeword is smaller than the expanding degree.
        let stopping_g0_len = cmp::max(params.lenghth_threshold_g0s, params.degree_g0);

        while g0_output_starts - g0_input_starts > stopping_g0_len {
            let n = g0_output_starts - g0_input_starts;
            let g0_output_len = (n as f64 * params.alpha_g0).round() as usize;

            g0s.push(OrionExpanderGraphPositioned::new(
                g0_input_starts,
                g0_output_starts,
                g0_output_starts + g0_output_len - 1,
                params.degree_g0,
                &mut rng,
            ));

            recursive_code_msg_code_starts.push((g0_input_starts, g0_output_starts));

            (g0_input_starts, g0_output_starts) =
                (g0_output_starts, g0_output_starts + g0_output_len);
        }

        // After g0s are generated, we generate g1s
        let mut g1_output_starts = g0_output_starts;

        while let Some((code_starts, g1_input_starts)) = recursive_code_msg_code_starts.pop() {
            let code_input_len = g1_input_starts - code_starts;
            let code_len = (code_input_len as f64 * params.inv_code_rate()).round() as usize;

            g1s.push(OrionExpanderGraphPositioned::new(
                g1_input_starts,
                g1_output_starts,
                code_starts + code_len - 1,
                params.degree_g1,
                &mut rng,
            ));

            g1_output_starts = code_starts + code_len;
        }

        Self { params, g0s, g1s }
    }

    #[inline(always)]
    pub fn code_len(&self) -> usize {
        self.params.output_code_len
    }

    #[inline(always)]
    pub fn msg_len(&self) -> usize {
        self.params.input_message_len
    }

    #[inline(always)]
    pub fn hamming_weight(&self) -> f64 {
        self.params.hamming_weight()
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

// NOTE we assume that the matrix has sides of length po2
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

/**********************************************************
 * IMPLEMENTATIONS FOR ORION POLYNOMIAL COMMITMENT SCHEME *
 **********************************************************/

pub const ORION_PCS_SOUNDNESS_BITS: usize = 128;

#[derive(Clone)]
pub struct OrionPCSImpl {
    pub num_variables: usize,

    pub code_instance: OrionCode,
}

#[derive(Clone)]
pub struct OrionCommitmentWithData<F: Field + FieldSerde, PackF: SimdField<Scalar = F>> {
    pub num_of_variables: usize,
    pub interleaved_codewords: Vec<F>,

    pub interleaved_alphabet_trees: Vec<Tree>,
    pub commitment_tree: Tree,

    pub _phantom: PhantomData<PackF>,
}

pub type OrionCommitment = tree::Node;

type OrionProximityCodeword<F> = Vec<F>;

pub struct OrionProof<F: Field + FieldSerde, ExtF: ExtensionField<BaseField = F>> {
    pub eval_row: Vec<ExtF>,
    pub proximity_rows: Vec<OrionProximityCodeword<ExtF>>,

    pub query_openings: Vec<(tree::Path, tree::Tree)>,
}

impl OrionPCSImpl {
    #[inline(always)]
    pub(crate) fn row_col_from_variables(num_variables: usize) -> (usize, usize) {
        let poly_variables: usize = num_variables;

        // NOTE(Hang): rounding up here in halving the poly variable num
        // up to discussion if we want to half by round down
        let row_num: usize = 1 << ((poly_variables + 1) / 2);
        let msg_size: usize = (1 << poly_variables) / row_num;

        (row_num, msg_size)
    }

    pub fn new(num_variables: usize, code_instance: OrionCode) -> OrionResult<Self> {
        let (_, msg_size) = Self::row_col_from_variables(num_variables);
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

    pub fn from_random(
        num_variables: usize,
        // TODO: should be removed with a precomputed list of params
        code_params: OrionCodeParameter,
        mut rng: impl rand::RngCore,
    ) -> OrionResult<Self> {
        let (_, msg_size) = Self::row_col_from_variables(num_variables);
        if msg_size != code_params.input_message_len {
            return Err(OrionPCSError::ParameterUnmatchError);
        }

        Ok(Self {
            num_variables,
            code_instance: OrionCode::new(code_params, &mut rng),
        })
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

    pub fn commit<F: Field + FieldSerde, PackF: SimdField<Scalar = F>>(
        &self,
        poly: &MultiLinearPoly<F>,
    ) -> OrionResult<OrionCommitmentWithData<F, PackF>> {
        let (row_num, msg_size) = Self::row_col_from_variables(poly.get_num_vars());

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

        // NOTE: unpack the packed codewords
        let interleaved_codewords: Vec<F> = packed_interleaved_codewords
            .iter()
            .flat_map(|p| p.unpack())
            .collect();

        // NOTE: commit the interleaved codeword
        // make sure that there are power-of-two number of leaves
        assert!((PackF::FIELD_SIZE * packed_rows / LEAF_BYTES).is_power_of_two());
        let interleaved_alphabet_trees: Vec<_> = packed_interleaved_codewords
            .chunks(packed_rows)
            .map(|packed_interleaved_alphabet| {
                let serialized_alphabet: Vec<_> = packed_interleaved_alphabet
                    .iter()
                    .map(|&p| {
                        let mut buffer = Vec::new();
                        p.serialize_into(&mut buffer)?;
                        Ok(buffer)
                    })
                    .collect::<OrionResult<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect();

                let leaves = serialized_alphabet
                    .chunks(LEAF_BYTES)
                    .map(|chunk| Leaf::new(chunk.try_into().unwrap()))
                    .collect();

                Ok(Tree::new_with_leaves(leaves))
            })
            .collect::<OrionResult<_>>()?;

        let column_size_to_po2 = interleaved_alphabet_trees.len().next_power_of_two();
        let mut commitment_leaves = vec![Leaf::default(); column_size_to_po2];
        interleaved_alphabet_trees
            .iter()
            .zip(commitment_leaves.iter_mut())
            .for_each(|(tree, leaf): (&Tree, &mut Leaf)| {
                let root = tree.root();
                leaf.data[..LEAF_HASH_BYTES].copy_from_slice(root.as_bytes());
            });
        let commitment_tree = Tree::new_with_leaves(commitment_leaves);

        Ok(OrionCommitmentWithData {
            num_of_variables: poly.get_num_vars(),
            interleaved_codewords,

            interleaved_alphabet_trees,
            commitment_tree,

            _phantom: PhantomData,
        })
    }

    pub fn open<F, PackF, ExtF, T>(
        &self,
        poly: &MultiLinearPoly<F>,
        #[allow(unused)] commitment_with_data: &OrionCommitmentWithData<F, PackF>,
        point: &[ExtF],
        transcript: &mut T,
    ) -> OrionProof<F, ExtF>
    where
        F: Field + FieldSerde,
        PackF: SimdField<Scalar = F>,
        ExtF: ExtensionField<BaseField = F>,
        T: Transcript<ExtF>,
    {
        let (row_num, msg_size) = Self::row_col_from_variables(poly.get_num_vars());
        let num_of_vars_in_codeword = log2(msg_size) as usize;

        // NOTE: working on evaluation response of tensor code IOP based PCS
        let mut poly_ext_field =
            MultiLinearPoly::new(poly.coeffs.iter().cloned().map(ExtF::from).collect());
        point[num_of_vars_in_codeword..]
            .iter()
            .rev()
            .for_each(|pt| poly_ext_field.fix_top_variable(pt));
        let eval_row = poly_ext_field.coeffs;

        // NOTE: transpose evaluations for random linear combination in proximity tests
        let mut transposed_evaluations = poly.coeffs.clone();
        let mut scratch = vec![F::ZERO; 1 << poly.get_num_vars()];
        transpose_in_place(&mut transposed_evaluations, &mut scratch, row_num);
        drop(scratch);

        // NOTE: draw random linear combination out
        // and compose proximity response(s) of tensor code IOP based PCS
        let proximity_repetitions =
            self.proximity_repetition_num(ORION_PCS_SOUNDNESS_BITS, ExtF::FIELD_SIZE);
        let proximity_rows: Vec<_> = (0..proximity_repetitions)
            .map(|_| {
                let random_linear_combination =
                    transcript.generate_challenge_field_elements(row_num);
                transposed_evaluations
                    .chunks(row_num)
                    .map(|col| {
                        col.iter()
                            .zip(random_linear_combination.iter())
                            .map(|(c, &li)| li.mul_by_base_field(c))
                            .sum()
                    })
                    .collect::<Vec<ExtF>>()
            })
            .collect();

        // NOTE: MT opening for point queries
        let query_num = self.query_complexity(ORION_PCS_SOUNDNESS_BITS);
        let mut query_points = transcript.generate_challenge_index_vector(query_num);
        let query_openings = query_points
            .iter_mut()
            .map(|qi| {
                *qi %= self.code_len();
                let path = commitment_with_data.commitment_tree.index_query(*qi);

                (
                    path,
                    commitment_with_data.interleaved_alphabet_trees[*qi].clone(),
                )
            })
            .collect();

        OrionProof {
            eval_row,
            proximity_rows,
            query_openings,
        }
    }

    pub fn verify<F, PackF, ExtF, T>(
        &self,
        commitment: &OrionCommitment,
        point: &[ExtF],
        evaluation: &ExtF,
        proof: &OrionProof<F, ExtF>,
        transcript: &mut T,
    ) -> bool
    where
        F: Field + FieldSerde,
        PackF: SimdField<Scalar = F>,
        ExtF: ExtensionField<BaseField = F>,
        T: Transcript<ExtF>,
    {
        let (row_num, msg_size) = Self::row_col_from_variables(point.len());
        let num_of_vars_in_codeword = log2(msg_size) as usize;

        // NOTE: working on evaluation response, evaluate the rest of the response
        let poly_half_evaled = MultiLinearPoly::new(proof.eval_row.clone());
        let final_eval = poly_half_evaled.evaluate_jolt(&point[..num_of_vars_in_codeword]);
        if final_eval != *evaluation {
            return false;
        }

        // NOTE: working on proximity responses, draw random linear combinations
        // then draw query points from fiat shamir transcripts
        let proximity_test_num =
            self.proximity_repetition_num(ORION_PCS_SOUNDNESS_BITS, ExtF::FIELD_SIZE);
        let random_linear_combinations: Vec<Vec<ExtF>> = (0..proximity_test_num)
            .map(|_| transcript.generate_challenge_field_elements(row_num))
            .collect();
        let query_num = self.query_complexity(ORION_PCS_SOUNDNESS_BITS);
        let mut query_points = transcript.generate_challenge_index_vector(query_num);
        query_points.iter_mut().for_each(|qi| {
            *qi %= self.code_len();
        });

        // NOTE: encode proximity responses and evaluation response
        let proximity_response_codewords: Vec<_> = match proof
            .proximity_rows
            .iter()
            .map(|r| self.code_instance.encode(r))
            .collect::<OrionResult<_>>()
        {
            Ok(codewords) => codewords,
            _ => return false,
        };
        let eval_response_codeword: OrionCodeword<ExtF> =
            match self.code_instance.encode(&proof.eval_row) {
                Ok(codeword) => codeword,
                _ => return false,
            };

        // NOTE: againts all index challenges, check alphabets against proximity responses
        // and evaluation response
        let eq_linear_combination = EqPolynomial::build_eq_x_r(&point[num_of_vars_in_codeword..]);

        random_linear_combinations
            .iter()
            .zip(proximity_response_codewords.iter())
            .chain(once((&eq_linear_combination, &eval_response_codeword)))
            .all(|(rl, codeword)| {
                query_points
                    .iter()
                    .zip(proof.query_openings.iter())
                    .all(|(&qi, (path, tree))| {
                        let bytes: Vec<_> = tree
                            .leaves
                            .iter()
                            .flat_map(|l| l.data.iter().cloned())
                            .collect();

                        let deserialized_leaves = bytes
                            .chunks(LEAF_BYTES)
                            .map(|chunk| Leaf::new(chunk.try_into().unwrap()))
                            .collect();

                        let computed_tree = Tree::new_with_leaves(deserialized_leaves);
                        if computed_tree.root() != tree.root() {
                            return false;
                        }

                        let interleaved_alphabet: Vec<_> = match bytes
                            .chunks(PackF::FIELD_SIZE / 8)
                            .map(|byte_slice| {
                                let pack = PackF::deserialize_from(byte_slice)?;
                                Ok(pack.unpack())
                            })
                            .collect::<FieldSerdeResult<Vec<_>>>()
                        {
                            Ok(col) => col.into_iter().flatten().collect(),
                            _ => return false,
                        };

                        let alphabet: ExtF = interleaved_alphabet
                            .iter()
                            .zip(rl.iter())
                            .map(|(c, r)| r.mul_by_base_field(c))
                            .sum();

                        alphabet == codeword[qi]
                            && path.verify(commitment)
                            && qi == path.index
                            && path.leaf.data[..LEAF_HASH_BYTES]
                                == tree.root().as_bytes()[..LEAF_HASH_BYTES]
                    })
            })
    }
}

// TODO waiting on a unified multilinear PCS trait - align OrionPCSImpl against PCS trait