//! Orion polynomial commitment scheme prototype implementaiton.
//! Includes implementation for Orion Expander-Code.

use arith::Field;
use rand::seq::index;

#[derive(Debug, Clone, Copy)]
pub struct OrionCodeParameter {
    pub rate_inv: f64,

    pub message_len: usize,

    // TODO: find a better name for threshold for (\alpha n)
    // maybe sub-message-len-threshold?
    pub length_threshold: usize,

    // parameter for graph g0, that maps n -> (\alpha n)
    pub alpha: f64,
    pub degree_g0: usize,

    // parameter for graph g1, let the message in the middle has length L,
    // then the graph g1 maps L -> ((rate_inv - 1) x n) - L
    pub degree_g1: usize,
}

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
    pub weighted_neighborings: Vec<Neighboring<F>>,

    // L R vertices size book keeping:
    // keep track of message length (l), and "compressed" code length (r)
    pub l_vertices_size: usize,
    pub r_vertices_size: usize,
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

// TODO: Orion code ascii code explanation for g0s and g1s, how they encode msg
#[derive(Clone)]
pub struct OrionCode<F: Field> {
    pub params: OrionCodeParameter,

    pub g0s: Vec<OrionExpanderGraph<F>>,
    pub g1s: Vec<OrionExpanderGraph<F>>,
}

impl<F: Field> OrionCode<F> {
    // TODO: generation new instance of orion code
    pub fn new(
        #[allow(unused)] params: OrionCodeParameter,
        #[allow(unused)] mut rng: impl rand::RngCore,
    ) -> Self {
        todo!()
    }

    // TODO: encode from expander graphs
    // length from n -> (rate_inv x n)
    pub fn encode(&self, msg: &[F]) -> Vec<F> {
        // TODO: error propagation for Orion encoding
        assert_eq!(msg.len(), self.params.message_len);

        todo!()
    }
}
