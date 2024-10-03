use ark_std::log2;
use p3_baby_bear::PackedBabyBearAVX512 as Babybearx16;

#[derive(Debug, Clone)]
pub struct MultiLinearPoly {
    pub coeffs: Vec<Babybearx16>,
}

impl MultiLinearPoly {
    pub fn get_num_vars(&self) -> usize {
        log2(self.coeffs.len()) as usize
    }

    // TODO: optimize this function
    pub fn interpolate_over_hypercube_impl(evals: &[Babybearx16]) -> Vec<Babybearx16> {
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
                    .for_each(|(a, b)| *a = *a - *b);
            })
        }

        coeffs
    }
}
