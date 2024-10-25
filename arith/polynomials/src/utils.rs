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
