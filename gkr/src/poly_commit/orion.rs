//! Orion polynomial commitment scheme prototype implementaiton.
//! Includes implementation for Orion Expander-Code.

use arith::Field;
use polynomials::MultiLinearPoly;
use rand::seq::index;
use thiserror::Error;

/******************************
 * PCS ERROR AND RESULT SETUP *
 ******************************/

#[derive(Debug, Error)]
pub enum OrionPCSError {
    #[error("Orion PCS linear code parameter unmatch error")]
    ParameterUnmatchError,
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
}

// TODO: fix a set of Orion code parameters for message length ranging 2^5 - 2^15

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

// TODO: Orion code ascii code explanation for g0s and g1s, how they encode msg
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

        while g0_output_starts - g0_input_starts > params.lenghth_threshold_g0s {
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
pub(crate) fn transpose_in_place<F: Field>(mat: &mut [F], scratch: &mut [F], row_num: usize) {
    let col_num = mat.len() / row_num;
    let batch_size = cache_batch_size::<F>();

    mat.chunks(batch_size)
        .enumerate()
        .for_each(|(i, ith_batch)| {
            let src_starts = i * batch_size;
            let dst_starts = (src_starts / col_num) + (src_starts % col_num) * row_num;

            ith_batch
                .iter()
                .enumerate()
                .for_each(|(j, &elem_j)| scratch[dst_starts + j * row_num] = elem_j)
        });

    mat.copy_from_slice(scratch);
}

/**********************************************************
 * IMPLEMENTATIONS FOR ORION POLYNOMIAL COMMITMENT SCHEME *
 **********************************************************/

#[derive(Clone)]
pub struct OrionPCSImpl {
    pub num_variables: usize,

    pub code_instance: OrionCode,
}

impl OrionPCSImpl {
    fn row_col_from_variables(num_variables: usize) -> (usize, usize) {
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

    // TODO query complexity for how many queries one need for interleaved codeword
    pub fn query_complexity(&self, #[allow(unused)] soundness_bits: usize) -> usize {
        todo!()
    }

    // TODO commitment with data
    pub fn commit<F: Field>(&self, poly: &MultiLinearPoly<F>) -> OrionResult<()> {
        let (row_num, msg_size) = Self::row_col_from_variables(poly.get_num_vars());

        // NOTE(Hang): another idea - if the inv_code_rate happens to be a po2
        // then it would very much favor us, as matrix will be square,
        // or composed by 2 squared matrices

        let mut interleaved_codeword_buffer =
            vec![F::ZERO; row_num * self.code_instance.code_len()];

        // NOTE: now the interleaved codeword is k x n matrix from expander code
        poly.coeffs
            .chunks(msg_size)
            .zip(interleaved_codeword_buffer.chunks_mut(self.code_instance.msg_len()))
            .try_for_each(|(row_i, codeword_i)| {
                self.code_instance.encode_in_place(row_i, codeword_i)
            })?;

        // NOTE: the interleaved codeword buffer is n x k matrix
        // with each column being an expander code
        let mut scratch = vec![F::ZERO; row_num * self.code_instance.code_len()];
        transpose_in_place(&mut interleaved_codeword_buffer, &mut scratch, row_num);
        drop(scratch);

        // TODO need a merkle tree to commit against all merkle tree roots

        todo!()
    }

    // TODO fiat-shamir challenge
    // TODO random evaluation point
    // TODO define orion proof structure
    pub fn open() {
        todo!()
    }

    // TODO after open gets implemented
    pub fn verify() {
        todo!()
    }

    // TODO after commit and open
    pub fn batch_commit() {
        todo!()
    }

    // TODO after commit and open
    pub fn batch_open() {
        todo!()
    }

    // TODO after commit and open
    pub fn batch_verify() {
        todo!()
    }
}

// TODO waiting on a unified multilinear PCS trait - align OrionPCSImpl against PCS trait
