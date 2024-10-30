//! Orion polynomial commitment scheme prototype implementaiton.
//! Includes implementation for Orion Expander-Code.

use arith::Field;
use rand::seq::index;

/********************************************
 * IMPLEMENTATIONS FOR ORION EXPANDER GRAPH *
 ********************************************/

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightedEdge<F: Field> {
    pub index: usize,
    pub weight: F,
}

impl<F: Field> WeightedEdge<F> {
    pub fn new(index: usize, mut rng: impl rand::RngCore) -> Self {
        Self {
            index,
            weight: F::random_unsafe(&mut rng),
        }
    }
}

type Neighboring<F> = Vec<WeightedEdge<F>>;

#[derive(Clone)]
pub struct OrionExpanderGraph<F: Field> {
    // L R vertices size book keeping:
    // keep track of message length (l), and "compressed" code length (r)
    pub l_vertices_size: usize,
    pub r_vertices_size: usize,

    // neighboring stands for all (weighted) connected vertices of a vertex.
    // In this context, the weighted_neighborings stands for the neighborings
    // of vertices in R set of the bipariate graph, which explains why it has
    // size of l_vertices_size, while each neighboring reserved r_vertices_size
    // capacity.
    pub weighted_neighborings: Vec<Neighboring<F>>,
}

impl<F: Field> OrionExpanderGraph<F> {
    pub fn new(
        l_vertices_size: usize,
        r_vertices_size: usize,
        expanding_degree: usize,
        mut rng: impl rand::RngCore,
    ) -> Self {
        let mut weighted_neighborings: Vec<Neighboring<F>> =
            vec![Vec::with_capacity(l_vertices_size); r_vertices_size];

        (0..l_vertices_size).for_each(|l_index| {
            let random_r_vertices = index::sample(&mut rng, r_vertices_size, expanding_degree);

            random_r_vertices.iter().for_each(|r_index| {
                weighted_neighborings[r_index].push(WeightedEdge::new(l_index, &mut rng))
            })
        });

        Self {
            weighted_neighborings,
            l_vertices_size,
            r_vertices_size,
        }
    }

    pub fn expander_mul(&self, l_vertices: &[F], r_vertices: &mut [F]) {
        // TODO: error propagation for Orion encoding
        assert_eq!(l_vertices.len(), self.l_vertices_size);
        assert_eq!(r_vertices.len(), self.r_vertices_size);

        r_vertices
            .iter_mut()
            .zip(self.weighted_neighborings.iter())
            .for_each(|(ri, ni)| {
                *ri = ni
                    .iter()
                    .map(|WeightedEdge { index, weight }| l_vertices[*index] * weight)
                    .sum();
            });
    }
}

/******************************************************
 * IMPLEMENTATIONS FOR ORION CODE FROM EXPANDER GRAPH *
 ******************************************************/

#[derive(Debug, Clone, Copy)]
pub struct OrionCodeParameter {
    // empirical parameters for the expander code on input/output size
    // NOTE: the derived code rate and invert code rate should preserve
    // in the recursive code of smaller size that comes in later rounds
    pub input_message_len: usize,
    pub output_code_len: usize,

    // parameter for graph g0, that maps n -> (\alpha n)
    // alpha should be ranging in (0, 1)
    pub alpha: f64,
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
    pub fn code_rate(&self) -> f64 {
        self.input_message_len as f64 / self.output_code_len as f64
    }

    pub fn inv_code_rate(&self) -> f64 {
        self.output_code_len as f64 / self.input_message_len as f64
    }
}

#[derive(Clone)]
pub struct OrionExpanderGraphPositioned<F: Field> {
    pub graph: OrionExpanderGraph<F>,

    pub input_starts: usize,
    pub output_starts: usize,
    pub output_ends: usize,
}

impl<F: Field> OrionExpanderGraphPositioned<F> {
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

    pub fn expander_mul_in_place(&self, buffer: &mut [F], scratch: &mut [F]) {
        let input_ref = &buffer[self.input_starts..self.output_starts];
        let output_ref = &mut scratch[self.output_starts..self.output_ends + 1];

        self.graph.expander_mul(input_ref, output_ref);
        buffer[self.output_starts..self.output_ends + 1].copy_from_slice(output_ref);
    }
}

// TODO: Orion code ascii code explanation for g0s and g1s, how they encode msg
#[derive(Clone)]
pub struct OrionCode<F: Field> {
    pub params: OrionCodeParameter,

    // g0s (affecting left side alphabets of the codeword)
    // generated from the largest to the smallest
    pub g0s: Vec<OrionExpanderGraphPositioned<F>>,

    // g1s (affecting right side alphabets of the codeword)
    // generated from the smallest to the largest
    pub g1s: Vec<OrionExpanderGraphPositioned<F>>,
}

impl<F: Field> OrionCode<F> {
    // TODO: generation new instance of orion code
    pub fn new(params: OrionCodeParameter, mut rng: impl rand::RngCore) -> Self {
        let mut recursive_code_msg_code_starts: Vec<(usize, usize)> = Vec::new();

        let mut g0s: Vec<OrionExpanderGraphPositioned<F>> = Vec::new();
        let mut g1s: Vec<OrionExpanderGraphPositioned<F>> = Vec::new();

        let mut g0_input_starts = 0;
        let mut g0_output_starts = params.input_message_len;

        while g0_output_starts - g0_input_starts > params.lenghth_threshold_g0s {
            let n = g0_output_starts - g0_input_starts;
            let g0_output_len = (n as f64 * params.alpha).round() as usize;

            g0s.push(OrionExpanderGraphPositioned::new(
                g0_input_starts,
                g0_output_starts,
                g0_output_starts + g0_output_len - 1,
                params.degree_g0,
                &mut rng,
            ));

            recursive_code_msg_code_starts.push((g0_input_starts, g0_output_starts));

            g0_input_starts = g0_output_starts;
            g0_output_starts += g0_output_len;
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

    pub fn code_len(&self) -> usize {
        self.params.output_code_len
    }

    pub fn msg_len(&self) -> usize {
        self.params.input_message_len
    }

    pub fn encode(&self, msg: &[F]) -> Vec<F> {
        // TODO: error propagation for Orion encoding
        assert_eq!(msg.len(), self.params.input_message_len);

        let mut codeword = vec![F::ZERO; self.code_len()];
        codeword[..self.msg_len()].copy_from_slice(msg);

        let mut scratch = vec![F::ZERO; self.code_len()];

        self.g0s
            .iter()
            .chain(self.g1s.iter())
            .for_each(|g| g.expander_mul_in_place(&mut codeword, &mut scratch));

        codeword
    }
}
