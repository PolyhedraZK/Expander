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
