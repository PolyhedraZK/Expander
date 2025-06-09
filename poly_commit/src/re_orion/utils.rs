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


// TODO: delete it
use std::time::{Instant, Duration};

pub struct Timer {
    start: Instant,
    last: Duration,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            start: Instant::now(),
            last: Duration::new(0, 0),
        }
    }

    pub fn count(&mut self) -> Duration {
        let e = self.start.elapsed();
        let res = e - self.last;
        self.last = e;
        res
    }
}