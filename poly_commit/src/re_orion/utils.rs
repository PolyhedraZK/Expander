pub fn transpose<T: Copy>(src: &[T], dst: &mut [T], n: usize, m: usize, block_size: usize) {
    for i in (0..m).step_by(block_size) {
        for j in (0..n).step_by(block_size) {
            for o in i..(i + block_size).min(m) {
                for p in j..(j + block_size).min(n) {
                    dst[p * m + o] = src[o * n + p];
                }
            }
        }
    }
}