use std::marker::PhantomData;

use arith::Field;

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

// TODO: Orion Expander Graph Struct
#[derive(Clone, Copy)]
pub struct OrionExpanderGraph<F: Field> {
    // TODO: neighborings: L to R, R to L
    // TODO: random weights: L to R, R to L

    // TODO: implement weights, and remove this
    _phantom: PhantomData<F>,
}

#[derive(Clone, Copy)]
pub struct OrionCode<F: Field> {
    pub params: OrionCodeParameter,

    // TODO: a bunch of graphs: [g0], [g1]

    // TODO: phantom marker for the field that the code operates on.
    // remove after graph is implemented
    _phantom: PhantomData<F>,
}

impl<F: Field> OrionCode<F> {
    // TODO: parameter generation
    pub fn new(
        #[allow(unused)] params: OrionCodeParameter,
        #[allow(unused)] mut rng: impl rand::RngCore,
    ) -> Self {
        todo!()
    }

    // TODO: encode from expander graphs
    // length from n -> (rate_inv x n)
    pub fn encode(&self, #[allow(unused)] msg: &[F]) -> Vec<F> {
        todo!()
    }
}
