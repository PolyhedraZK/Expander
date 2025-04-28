use arith::{bit_reverse, Field};
use gkr_engine::MPIEngine;

#[allow(unused)]
#[inline(always)]
pub(crate) fn mpi_reed_solomon_encoding<F: Field>(
    mpi_config: &impl MPIEngine,
    coeffs: &[F],
    code_rate_log2_bits: usize,
) {
    let mut codeword: Vec<F> = coeffs.to_vec();

    // NOTE(HS) bit inverse and FFT for MPI world variables
    {
        let mut root_codeword = {
            let global_len = coeffs.len() * mpi_config.world_size();
            vec![F::ZERO; if mpi_config.is_root() { global_len } else { 0 }]
        };

        mpi_config.gather_vec(&codeword, &mut root_codeword);

        let num_world_bits = (mpi_config.world_size() - 1).count_ones();

        // NOTE(HS) only the root process runs the bit inverse and FFT
        if mpi_config.is_root() {
            // Bit inverse chunks of world
            (0..mpi_config.world_size()).for_each(|i| {
                let swap_to = bit_reverse(i, num_world_bits as usize);
                if i >= swap_to {
                    return;
                }

                for j in 0..codeword.len() {
                    root_codeword.swap(i * codeword.len() + j, swap_to * codeword.len() + j);
                }
            });

            let mut chunk = 2 * coeffs.len();
            for _ in 0..num_world_bits {
                // TODO(HS) FFT for MPI world variables
                root_codeword.chunks_mut(chunk).for_each(|coeffs| {
                    let (left, right) = coeffs.split_at_mut(chunk / 2);
                });

                chunk <<= 1;
            }
        }

        // NOTE finally scatter back into own world
        mpi_config.scatter_vec(&root_codeword, &mut codeword);
    }

    todo!()
}

#[cfg(test)]
mod fft_test {
    use arith::{bit_reverse_swap, Fr};

    #[test]
    fn test_fft_stuff() {
        let mut coeffs = vec![
            Fr::from(0u32),
            Fr::from(1u32),
            Fr::from(2u32),
            Fr::from(3u32),
        ];

        bit_reverse_swap(&mut coeffs);
        dbg!(&coeffs);

        coeffs.resize(16, Fr::from(0u32));

        bit_reverse_swap(&mut coeffs);
        dbg!(&coeffs);
    }
}
