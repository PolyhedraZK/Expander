use halo2curves::ff::Field;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

pub(crate) fn powers_of_field_elements<F: Field>(x: &F, n: usize) -> Vec<F> {
    let mut powers = vec![F::ONE];
    let mut cur = *x;
    for _ in 0..n - 1 {
        powers.push(cur);
        cur *= x;
    }
    powers
}

pub(crate) fn tensor_product_parallel<F: Field>(vec1: &[F], vec2: &[F]) -> Vec<F> {
    vec2.par_iter()
        .flat_map(|&i| vec1.iter().map(|&j| i * j).collect::<Vec<_>>())
        .collect()
}

/// For x in points, compute the Lagrange coefficients at x given the roots.
/// `L_{i}(x) = \prod_{j \neq i} \frac{x - r_j}{r_i - r_j}``
pub(crate) fn lagrange_coefficients<F: Field + Send + Sync>(roots: &[F], points: &[F]) -> Vec<F> {
    roots
        .par_iter()
        .enumerate()
        .map(|(i, _)| {
            let mut numerator = F::ONE;
            let mut denominator = F::ONE;
            for j in 0..roots.len() {
                if i == j {
                    continue;
                }
                numerator *= roots[j] - points[i];
                denominator *= roots[j] - roots[i];
            }
            numerator * denominator.invert().unwrap()
        })
        .collect()
}

/// This simple utility function will parallelize an operation that is to be
/// performed over a mutable slice.
/// credit: https://github.com/scroll-tech/halo2/blob/1070391642dd64b2d68b47ec246cba9e35bd3c15/halo2_proofs/src/arithmetic.rs#L546
pub(crate) fn parallelize_internal<T: Send, F: Fn(&mut [T], usize) + Send + Sync + Clone>(
    v: &mut [T],
    f: F,
) -> Vec<usize> {
    let n = v.len();
    let num_threads = rayon::current_num_threads();
    let mut chunk = (n as usize) / num_threads;
    if chunk < num_threads {
        chunk = 1;
    }

    rayon::scope(|scope| {
        let mut chunk_starts = vec![];
        for (chunk_num, v) in v.chunks_mut(chunk).enumerate() {
            let f = f.clone();
            scope.spawn(move |_| {
                let start = chunk_num * chunk;
                f(v, start);
            });
            let start = chunk_num * chunk;
            chunk_starts.push(start);
        }

        chunk_starts
    })
}

pub fn parallelize<T: Send, F: Fn(&mut [T], usize) + Send + Sync + Clone>(v: &mut [T], f: F) {
    parallelize_internal(v, f);
}

#[test]
fn test_lagrange_eval() {
    use halo2curves::bn256::Fr;
    let roots = vec![Fr::from(1u64), Fr::from(2u64), Fr::from(3u64)];
    let points = vec![Fr::from(4u64), Fr::from(5u64), Fr::from(6u64)];
    let result = lagrange_coefficients(&roots, &points);
    assert_eq!(result[0], Fr::from(1u64));
    assert_eq!(result[1], -Fr::from(8u64));
    assert_eq!(result[2], Fr::from(10u64));
}


#[test]
fn test_tensor_product(){
    use halo2curves::bn256::Fr;
    let vec1 = vec![Fr::from(1u64), Fr::from(2u64), Fr::from(3u64)];
    let vec2 = vec![Fr::from(4u64), Fr::from(5u64), Fr::from(6u64)];
    let result = tensor_product_parallel(&vec1, &vec2);
    assert_eq!(result[0], Fr::from(4u64));
    assert_eq!(result[1], Fr::from(2u64)*Fr::from(4u64));
    assert_eq!(result[2], Fr::from(3u64)*Fr::from(4u64));
    assert_eq!(result[3], Fr::from(5u64));
    assert_eq!(result[4], Fr::from(2u64)*Fr::from(5u64));
    assert_eq!(result[5], Fr::from(3u64)*Fr::from(5u64));
    assert_eq!(result[6], Fr::from(6u64));
    assert_eq!(result[7], Fr::from(2u64)*Fr::from(6u64));
    assert_eq!(result[8], Fr::from(3u64)*Fr::from(6u64));
}

    
