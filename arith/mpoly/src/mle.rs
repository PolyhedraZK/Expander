use arith::Field;
use ark_std::{log2, rand::RngCore};

#[derive(Debug, Clone)]
pub struct MultiLinearPoly<F: Field> {
    pub coeffs: Vec<F>,
}

impl<F: Field> MultiLinearPoly<F> {
    /// Sample a random polynomials.
    pub fn random(nv: usize, mut rng: impl RngCore) -> Self {
        let coeff = (0..1 << nv).map(|_| F::random_unsafe(&mut rng)).collect();
        Self { coeffs: coeff }
    }

    pub fn get_num_vars(&self) -> usize {
        log2(self.coeffs.len()) as usize
    }

    // TODO: optimize this function
    pub fn interpolate_over_hypercube_impl(evals: &[F]) -> Vec<F> {
        let mut coeffs = evals.to_vec();
        let num_vars = log2(evals.len());

        for i in 1..=num_vars {
            let chunk_size = 1 << i;

            coeffs.chunks_mut(chunk_size).for_each(|chunk| {
                let half_chunk = chunk_size >> 1;
                let (left, right) = chunk.split_at_mut(half_chunk);
                right
                    .iter_mut()
                    .zip(left.iter())
                    .for_each(|(a, b)| *a -= *b);
            })
        }

        coeffs
    }

    // interpolate Z evaluations over boolean hypercube {0, 1}^n
    pub fn interpolate_over_hypercube(&self) -> Vec<F> {
        // Take eq poly as an example:
        //
        // The evaluation format of an eq poly over {0, 1}^2 follows:
        // eq(\vec{r}, \vec{x}) with \vec{x} order in x0 x1
        //
        //     00             01            10          11
        // (1-r0)(1-r1)    (1-r0)r1      r0(1-r1)      r0r1
        //
        // The interpolated version over x0 x1 (ordered in x0 x1) follows:
        //
        //     00               01                  10                11
        // (1-r0)(1-r1)    (1-r0)(2r1-1)      (2r0-1)(1-r1)     (2r0-1)(2r1-1)

        // NOTE(Hang): I think the original implementation of this dense multilinear
        // polynomial requires a resizing of coeffs by num vars,
        // e.g., when sumchecking - the num_var reduces, while Z evals can reuse the
        // whole space, which means we cannot simply relying Z's size itself.
        Self::interpolate_over_hypercube_impl(&self.coeffs)
    }

    // Evaluate the polynomial at a single variable
    pub fn fix_top_variable(&mut self, r: &F) {
        let n = self.coeffs.len() / 2;
        let (left, right) = self.coeffs.split_at_mut(n);

        left.iter_mut().zip(right.iter()).for_each(|(a, b)| {
            *a += *r * (*b - *a);
        });
        self.coeffs.truncate(n);
    }

    // Evaluate the polynomial at a set of variables
    pub fn fix_variables(&mut self, partial_point: &[F]) {
        // evaluate single variable of partial point from left to right
        partial_point
            .iter()
            .for_each(|point| self.fix_top_variable(point));
    }
}
