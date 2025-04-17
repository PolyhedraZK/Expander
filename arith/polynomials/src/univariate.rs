use std::ops::{AddAssign, Index, IndexMut, MulAssign};

use arith::{FFTField, Field};
use itertools::izip;

#[derive(Debug, Clone, Default)]
pub struct UnivariatePoly<F: Field> {
    pub coeffs: Vec<F>,
}

#[derive(Debug, Clone, Default)]
pub struct UnivariateLagrangePolynomial<F: FFTField> {
    pub evals: Vec<F>,
}

impl<F: Field> Index<usize> for UnivariatePoly<F> {
    type Output = F;

    fn index(&self, index: usize) -> &Self::Output {
        &self.coeffs[index]
    }
}

impl<F: Field> IndexMut<usize> for UnivariatePoly<F> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.coeffs[index]
    }
}

impl<F: FFTField> Index<usize> for UnivariateLagrangePolynomial<F> {
    type Output = F;

    fn index(&self, index: usize) -> &Self::Output {
        &self.evals[index]
    }
}

impl<F: FFTField> IndexMut<usize> for UnivariateLagrangePolynomial<F> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.evals[index]
    }
}

// Credit: code adopted from Jolt
//
// Jolt's ack:
//
// Inspired by: https://github.com/TheAlgorithms/Rust/blob/master/src/math/gaussian_elimination.rs
// Gaussian Elimination of Quadratic Matrices
// Takes an augmented matrix as input, returns vector of results
// Wikipedia reference: augmented matrix: https://en.wikipedia.org/wiki/Augmented_matrix
// Wikipedia reference: algorithm: https://en.wikipedia.org/wiki/Gaussian_elimination

pub fn gaussian_elimination<F: Field>(matrix: &mut [Vec<F>]) -> Vec<F> {
    let size = matrix.len();
    assert_eq!(size, matrix[0].len() - 1);

    for i in 0..size - 1 {
        for j in i..size - 1 {
            echelon(matrix, i, j);
        }
    }

    for i in (1..size).rev() {
        eliminate(matrix, i);
    }

    // Disable cargo clippy warnings about needless range loops.
    // Checking the diagonal like this is simpler than any alternative.
    #[allow(clippy::needless_range_loop)]
    for i in 0..size {
        if matrix[i][i] == F::zero() {
            println!("Infinitely many solutions");
        }
    }

    let mut result: Vec<F> = vec![F::zero(); size];
    for i in 0..size {
        result[i] = matrix[i][size] * matrix[i][i].inv().unwrap();
    }
    result
}

fn echelon<F: Field>(matrix: &mut [Vec<F>], i: usize, j: usize) {
    let size = matrix.len();
    if matrix[i][i] == F::zero() {
    } else {
        let factor = matrix[j + 1][i] * matrix[i][i].inv().unwrap();
        (i..size + 1).for_each(|k| {
            let tmp = matrix[i][k];
            matrix[j + 1][k] -= factor * tmp;
        });
    }
}

fn eliminate<F: Field>(matrix: &mut [Vec<F>], i: usize) {
    let size = matrix.len();
    if matrix[i][i] == F::zero() {
    } else {
        for j in (1..i + 1).rev() {
            let factor = matrix[j - 1][i] * matrix[i][i].inv().unwrap();
            for k in (0..size + 1).rev() {
                let tmp = matrix[i][k];
                matrix[j - 1][k] -= factor * tmp;
            }
        }
    }
}

fn vandermonde_interpolation<F>(evals: &[F]) -> Vec<F>
where
    F: Field,
{
    let n = evals.len();
    let xs: Vec<F> = (0..n).map(|x| F::from(x as u32)).collect();

    let mut vandermonde: Vec<Vec<F>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = Vec::with_capacity(n);
        let x = xs[i];
        row.push(F::one());
        row.push(x);
        for j in 2..n {
            row.push(row[j - 1] * x);
        }
        row.push(evals[i]);
        vandermonde.push(row);
    }

    gaussian_elimination(&mut vandermonde)
}

impl<F: Field> UnivariatePoly<F> {
    #[inline]
    pub fn new(coeffs: Vec<F>) -> Self {
        Self { coeffs }
    }

    /// Sample a random polynomials.
    #[inline]
    pub fn random(degree: usize, mut rng: impl rand::RngCore) -> Self {
        let coeffs = (0..=degree).map(|_| F::random_unsafe(&mut rng)).collect();
        Self { coeffs }
    }

    /// Evaluation by Horner's rule
    #[inline]
    pub fn evaluate(&self, point: F) -> F {
        let mut eval = F::ZERO;
        self.coeffs.iter().rev().for_each(|t| {
            eval *= point;
            eval += *t
        });
        eval
    }

    pub fn from_evals(evals: &[F]) -> Self {
        Self {
            coeffs: vandermonde_interpolation(evals),
        }
    }

    /// Performing a polynomial division by f(X) / (X - \alpha),
    /// returning quotient q(X) and remainder r satisfying q(X) (X - \alpha) + r == f(X).
    #[inline(always)]
    pub fn degree_one_quotient(&self, alpha: F) -> (Self, F) {
        let mut div_coeffs = self.coeffs.to_vec();

        for i in (1..self.coeffs.len()).rev() {
            // c X^n = c X^n - c \alpha X^(n - 1) + c \alpha X^(n - 1)
            //       = c (X - \alpha) X^(n - 1) + c \alpha X^(n - 1)

            let remainder = div_coeffs[i] * alpha;
            div_coeffs[i - 1] += remainder;
        }

        let final_remainder = div_coeffs[0];
        let mut final_div_coeffs = div_coeffs[1..].to_owned();
        final_div_coeffs.resize(self.coeffs.len(), F::ZERO);

        (Self::new(final_div_coeffs), final_remainder)
    }

    /// Performing a series of polynomial divisions f(X) / \prod_{r \in R} (X - r),
    /// where R is the roots.
    /// NOTE: we assume that f(X) vanishes over these roots.
    #[inline(always)]
    pub fn root_vanishing_quotient(&mut self, roots: &[F]) {
        roots.iter().enumerate().for_each(|(ith_root, r)| {
            for i in (1 + ith_root..self.coeffs.len()).rev() {
                // c X^n = c X^n - c \alpha X^(n - 1) + c \alpha X^(n - 1)
                //       = c (X - \alpha) X^(n - 1) + c \alpha X^(n - 1)

                let remainder = self.coeffs[i] * r;
                self.coeffs[i - 1] += remainder;
            }

            assert_eq!(self.coeffs[ith_root], F::ZERO);
        });

        self.coeffs.drain(0..roots.len());
        self.coeffs.resize(self.coeffs.len() + roots.len(), F::ZERO);
    }
}

impl<F: Field> MulAssign<F> for UnivariatePoly<F> {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: F) {
        self.coeffs.iter_mut().for_each(|c| *c *= rhs)
    }
}

impl<F: Field> AddAssign<&Self> for UnivariatePoly<F> {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &Self) {
        if rhs.coeffs.len() > self.coeffs.len() {
            self.coeffs.resize(rhs.coeffs.len(), F::ZERO);
        }

        izip!(&mut self.coeffs, &rhs.coeffs).for_each(|(c, r)| *c += r);
    }
}

impl<F: FFTField> UnivariateLagrangePolynomial<F> {
    #[inline]
    pub fn new(evals: Vec<F>) -> Self {
        assert!(evals.len().is_power_of_two());

        Self { evals }
    }

    #[inline]
    pub fn random(degree: usize, mut rng: impl rand::RngCore) -> Self {
        assert!((degree + 1).is_power_of_two());
        let evals = (0..=degree).map(|_| F::random_unsafe(&mut rng)).collect();
        Self { evals }
    }

    #[inline]
    pub fn evaluate(&self, point: F) -> F {
        let n = self.evals.len();
        let log_n = n.ilog2() as usize;
        let omega = F::two_adic_generator(log_n);
        let omega_inv = omega.inv().unwrap();

        let nominator_prepare = point.exp(n as u128) - F::ONE;
        let denominator_prepare = F::ONE.double().exp(log_n as u128);

        let mut omega_i = F::ONE;
        let mut denominator = denominator_prepare;

        self.evals
            .iter()
            .map(|e_i| {
                let nominator_vanisher = point - omega_i;

                let res = if nominator_vanisher.is_zero() {
                    *e_i
                } else if nominator_prepare.is_zero() {
                    F::ZERO
                } else {
                    let nominator = nominator_prepare * nominator_vanisher.inv().unwrap();
                    *e_i * nominator * denominator.inv().unwrap()
                };

                omega_i *= omega;
                denominator *= omega_inv;

                res
            })
            .sum()
    }
}

impl<F: FFTField> UnivariatePoly<F> {
    #[inline]
    pub fn fft(mut self) -> UnivariateLagrangePolynomial<F> {
        F::fft_in_place(&mut self.coeffs);

        UnivariateLagrangePolynomial::new(self.coeffs)
    }
}

impl<F: FFTField> UnivariateLagrangePolynomial<F> {
    #[inline]
    pub fn ifft(mut self) -> UnivariatePoly<F> {
        F::ifft_in_place(&mut self.evals);

        UnivariatePoly::new(self.evals)
    }
}

#[derive(Debug, Clone)]
pub struct EqUnivariatePoly<F: Field> {
    pub point: Vec<F>,
}

impl<F: Field> EqUnivariatePoly<F> {
    #[inline]
    pub fn new(point: Vec<F>) -> Self {
        Self { point }
    }

    #[inline]
    pub fn degree(&self) -> usize {
        (1 << self.point.len()) - 1
    }

    /// Evaluation in O(\log N) time
    #[inline]
    pub fn evaluate(&self, x: F) -> F {
        let mut eval = F::ONE;
        let mut x_pow = x;

        self.point.iter().for_each(|e| {
            eval *= x_pow * e + F::ONE - e;
            x_pow = x_pow.square();
        });

        eval
    }
}
