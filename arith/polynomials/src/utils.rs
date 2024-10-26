use arith::{FFTField, Field};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

#[inline]
pub fn powers_of_field_elements<F: Field>(x: &F, n: usize) -> Vec<F> {
    let mut powers = vec![F::ONE];
    let mut cur = *x;
    for _ in 0..n - 1 {
        powers.push(cur);
        cur *= x;
    }
    powers
}

#[inline]
pub fn tensor_product_parallel<F: Field>(vec1: &[F], vec2: &[F]) -> Vec<F> {
    vec2.par_iter()
        .flat_map(|&i| vec1.iter().map(|&j| i * j).collect::<Vec<_>>())
        .collect()
}

#[inline]
pub fn primitive_root_of_unity<F: FFTField>(group_size: usize) -> F {
    let log_size = group_size.trailing_zeros() as usize;
    assert_eq!(1 << log_size, group_size);
    F::two_adic_generator(log_size)
}

#[inline]
pub fn bitreverse(mut n: usize, l: usize) -> usize {
    let mut r = 0;
    for _ in 0..l {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}

#[inline]
pub fn batch_inversion_in_place<F>(values: &mut Vec<F>)
where
    F: Field + Copy,
{
    // Step 1: Compute running products
    // prod[i] = a[0] * a[1] * ... * a[i]
    let mut prod = vec![F::zero(); values.len()];
    prod[0] = values[0];

    for i in 1..values.len() {
        if values[i].is_zero() {
            panic!("Cannot invert a vector with a zero entry");
        }
        prod[i] = prod[i - 1] * values[i];
    }

    // Step 2: Invert the final product
    let mut inv = prod.last().unwrap().inv().unwrap();

    // Step 3: Walk backwards to compute individual inverses
    for i in (1..values.len()).rev() {
        let tmp = inv * prod[i - 1];
        inv *= values[i];
        values[i] = tmp;
    }
    values[0] = inv;
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
