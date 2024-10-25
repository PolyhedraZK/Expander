/// This simple utility function will parallelize an operation that is to be
/// performed over a mutable slice.
/// credit: https://github.com/scroll-tech/halo2/blob/1070391642dd64b2d68b47ec246cba9e35bd3c15/halo2_proofs/src/arithmetic.rs#L546
pub(crate) fn parallelize_internal<T: Send, F: Fn(&mut [T], usize) + Send + Sync + Clone>(
    v: &mut [T],
    f: F,
) -> Vec<usize> {
    let n = v.len();
    let num_threads = rayon::current_num_threads();
    let mut chunk = n / num_threads;
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
    if rayon::current_num_threads() == 1 {
        // do not spawn a new thread
        f(v, 0)
    } else {
        parallelize_internal(v, f);
    }
}
