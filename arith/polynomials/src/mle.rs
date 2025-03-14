use std::{
    cmp,
    marker::PhantomData,
    ops::{Index, IndexMut, Mul},
};

use arith::{Field, SimdField};
use ark_std::{log2, rand::RngCore};
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;

use crate::{EqPolynomial, MultilinearExtension, MutableMultilinearExtension};

#[derive(Debug, Clone, Default)]
pub struct MultiLinearPoly<F: Field> {
    pub coeffs: Vec<F>,
}

impl<F: Field> MultiLinearPoly<F> {
    #[inline]
    pub fn new(evals: Vec<F>) -> Self {
        assert!(evals.len().is_power_of_two());

        Self { coeffs: evals }
    }

    /// Sample a random polynomials.
    #[inline]
    pub fn random(nv: usize, mut rng: impl RngCore) -> Self {
        let coeff = (0..1 << nv).map(|_| F::random_unsafe(&mut rng)).collect();
        Self { coeffs: coeff }
    }

    #[inline]
    pub fn get_num_vars(&self) -> usize {
        log2(self.coeffs.len()) as usize
    }

    // TODO: optimize this function
    #[inline]
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
    #[inline]
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

    /// Evaluate the polynomial at the top variable
    #[inline]
    pub fn fix_top_variable<AF: Field + Mul<F, Output = F>>(&mut self, r: AF) {
        let n = self.coeffs.len() / 2;
        let (left, right) = self.coeffs.split_at_mut(n);

        left.iter_mut().zip(right.iter()).for_each(|(a, b)| {
            *a += r * (*b - *a);
        });
        self.coeffs.truncate(n);
    }

    /// Hyperplonk's implementation
    /// Evaluate the polynomial at a set of variables, from bottom to top
    /// This is equivalent to `evaluate` when partial_point.len() = nv
    #[inline]
    pub fn fix_variables<AF: Field + Mul<F, Output = F>>(&mut self, partial_point: &[AF]) {
        // evaluate single variable of partial point from left to right
        partial_point
            .iter()
            .rev() // need to reverse the order of the point
            .for_each(|point| self.fix_top_variable(*point));
    }

    /// Jolt's implementation
    /// Evaluate the polynomial at a set of variables, from bottom to top
    /// This is equivalent to `evaluate_with_buffer`, but slower
    /// returns Z(r) in O(n) time
    #[inline]
    pub fn evaluate_jolt(&self, r: &[F]) -> F {
        // r must have a value for each variable
        assert_eq!(r.len(), self.get_num_vars());
        let chis = EqPolynomial::evals_jolt(r);
        assert_eq!(chis.len(), self.coeffs.len());
        self.coeffs
            .iter()
            .zip(chis.iter())
            .map(|(c, chi)| *c * *chi)
            .sum()
    }

    #[inline]
    /// Expander's implementation
    /// Generic method to evaluate a multilinear polynomial.
    /// This is the preferred method to evaluate a multilinear polynomial
    /// as it does not require additional memory.
    pub fn evaluate_with_buffer(evals: &[F], point: &[F], scratch: &mut [F]) -> F {
        assert_eq!(1 << point.len(), evals.len());
        assert!(scratch.len() >= evals.len());

        if point.is_empty() {
            evals[0]
        } else {
            for i in 0..(evals.len() >> 1) {
                scratch[i] = (evals[i * 2 + 1] - evals[i * 2]) * point[0] + evals[i * 2];
            }

            let mut cur_eval_size = evals.len() >> 2;
            for r in point.iter().skip(1) {
                for i in 0..cur_eval_size {
                    scratch[i] = scratch[i * 2] + (scratch[i * 2 + 1] - scratch[i * 2]) * r;
                }
                cur_eval_size >>= 1;
            }
            scratch[0]
        }
    }
}

impl<F: Field> Index<usize> for MultiLinearPoly<F> {
    type Output = F;

    fn index(&self, index: usize) -> &Self::Output {
        &self.coeffs[index]
    }
}

impl<F: Field> MultilinearExtension<F> for MultiLinearPoly<F> {
    fn num_vars(&self) -> usize {
        self.get_num_vars()
    }

    fn hypercube_basis(&self) -> Vec<F> {
        self.coeffs.clone()
    }

    fn hypercube_basis_ref(&self) -> &Vec<F> {
        &self.coeffs
    }

    fn evaluate_with_buffer(&self, point: &[F], scratch: &mut [F]) -> F {
        Self::evaluate_with_buffer(&self.coeffs, point, scratch)
    }

    fn interpolate_over_hypercube(&self) -> Vec<F> {
        self.interpolate_over_hypercube()
    }
}

impl<F: Field> IndexMut<usize> for MultiLinearPoly<F> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.coeffs[index]
    }
}

impl<F: Field> MutableMultilinearExtension<F> for MultiLinearPoly<F> {
    fn fix_top_variable<AF: Field + std::ops::Mul<F, Output = F>>(&mut self, r: AF) {
        self.fix_top_variable(r)
    }

    fn fix_variables<AF: Field + std::ops::Mul<F, Output = F>>(&mut self, vars: &[AF]) {
        self.fix_variables(vars)
    }

    fn interpolate_over_hypercube_in_place(&mut self) {
        let num_vars = self.num_vars();
        for i in 1..=num_vars {
            let chunk_size = 1 << i;

            self.coeffs.chunks_mut(chunk_size).for_each(|chunk| {
                let half_chunk = chunk_size >> 1;
                let (left, right) = chunk.split_at_mut(half_chunk);
                right
                    .iter_mut()
                    .zip(left.iter())
                    .for_each(|(a, b)| *a -= *b);
            })
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MultiLinearPolyExpander<C: GKRFieldConfig> {
    _config: PhantomData<C>,
}

/// Some dedicated mle implementations for GKRFieldConfig
/// Take into consideration the simd challenge and the mpi challenge
///
/// This is more efficient than the generic implementation by avoiding
/// unnecessary conversions between field types
impl<C: GKRFieldConfig> MultiLinearPolyExpander<C> {
    pub fn new() -> Self {
        Self {
            _config: PhantomData,
        }
    }

    #[inline]
    pub fn eval_circuit_vals_at_challenge(
        evals: &[C::SimdCircuitField],
        x: &[C::ChallengeField],
        scratch: &mut [C::Field],
    ) -> C::Field {
        assert_eq!(1 << x.len(), evals.len());
        assert!(scratch.len() >= evals.len());

        if x.is_empty() {
            C::simd_circuit_field_into_field(&evals[0])
        } else {
            for i in 0..(evals.len() >> 1) {
                scratch[i] = C::field_add_simd_circuit_field(
                    &C::simd_circuit_field_mul_challenge_field(
                        &(evals[i * 2 + 1] - evals[i * 2]),
                        &x[0],
                    ),
                    &evals[i * 2],
                );
            }

            let mut cur_eval_size = evals.len() >> 2;
            for r in x.iter().skip(1) {
                for i in 0..cur_eval_size {
                    scratch[i] = scratch[i * 2] + (scratch[i * 2 + 1] - scratch[i * 2]).scale(r);
                }
                cur_eval_size >>= 1;
            }
            scratch[0]
        }
    }

    /// This assumes each mpi core hold their own evals, and collectively
    /// compute the global evaluation.
    /// Mostly used by the prover run with `mpiexec`
    #[inline]
    pub fn collectively_eval_circuit_vals_at_expander_challenge(
        local_evals: &[C::SimdCircuitField],
        x: &[C::ChallengeField],
        x_simd: &[C::ChallengeField],
        x_mpi: &[C::ChallengeField],
        scratch_field: &mut [C::Field],
        scratch_challenge_field: &mut [C::ChallengeField],
        mpi_config: &MPIConfig,
    ) -> C::ChallengeField {
        assert!(scratch_challenge_field.len() >= 1 << cmp::max(x_simd.len(), x_mpi.len()));

        let local_simd = Self::eval_circuit_vals_at_challenge(local_evals, x, scratch_field);
        let local_simd_unpacked = local_simd.unpack();
        let local_v = MultiLinearPoly::evaluate_with_buffer(
            &local_simd_unpacked,
            x_simd,
            scratch_challenge_field,
        );

        if mpi_config.is_root() {
            let mut claimed_v_gathering_buffer =
                vec![C::ChallengeField::zero(); mpi_config.world_size()];
            mpi_config.gather_vec(&vec![local_v], &mut claimed_v_gathering_buffer);
            MultiLinearPoly::evaluate_with_buffer(
                &claimed_v_gathering_buffer,
                x_mpi,
                scratch_challenge_field,
            )
        } else {
            mpi_config.gather_vec(&vec![local_v], &mut vec![]);
            C::ChallengeField::zero()
        }
    }

    /// This assumes only a single core holds all the evals, and evaluate it locally
    /// mostly used by the verifier
    #[inline]
    pub fn single_core_eval_circuit_vals_at_expander_challenge(
        global_vals: &[C::SimdCircuitField],
        x: &[C::ChallengeField],
        x_simd: &[C::ChallengeField],
        x_mpi: &[C::ChallengeField],
    ) -> C::ChallengeField {
        let local_poly_size = global_vals.len() >> x_mpi.len();
        assert_eq!(local_poly_size, 1 << x.len());

        let mut scratch_field = vec![C::Field::default(); local_poly_size];
        let mut scratch_challenge_field =
            vec![C::ChallengeField::default(); 1 << cmp::max(x_simd.len(), x_mpi.len())];
        let local_evals = global_vals
            .chunks(local_poly_size)
            .map(|local_vals| {
                let local_simd =
                    Self::eval_circuit_vals_at_challenge(local_vals, x, &mut scratch_field);
                let local_simd_unpacked = local_simd.unpack();
                MultiLinearPoly::evaluate_with_buffer(
                    &local_simd_unpacked,
                    x_simd,
                    &mut scratch_challenge_field,
                )
            })
            .collect::<Vec<C::ChallengeField>>();

        let mut scratch = vec![C::ChallengeField::default(); local_evals.len()];
        MultiLinearPoly::evaluate_with_buffer(&local_evals, x_mpi, &mut scratch)
    }
}
