use arith::{bit_reverse, FFTField};
use gkr_engine::MPIEngine;

use crate::utils::mpi_matrix_transpose;

#[allow(unused)]
#[inline(always)]
pub(crate) fn mpi_reed_solomon_encoding<F: FFTField>(
    mpi_config: &impl MPIEngine,
    local_coeffs: &[F],
    code_rate_log2_bits: usize,
) {
    assert!(local_coeffs.len().is_power_of_two());

    let (col_size, two_adic_bits): (usize, usize) = {
        let hypercube_bits = local_coeffs.len().ilog2() + mpi_config.world_size().ilog2();
        let codeword_bits = hypercube_bits as usize + code_rate_log2_bits;
        if codeword_bits <= F::TWO_ADICITY {
            (1, codeword_bits)
        } else {
            (1 << (codeword_bits - F::TWO_ADICITY), F::TWO_ADICITY)
        }
    };
    let generator = F::two_adic_generator(two_adic_bits);

    let mut codeword: Vec<F> = local_coeffs.to_vec();

    mpi_matrix_transpose(mpi_config, &mut codeword, col_size);

    // TODO extend local codewords to sufficient size, run local fft with generator

    // TODO ... local FFT stuffs

    // NOTE(HS) bit inverse and FFT for MPI world variables
    {
        let mut root_codeword = {
            let global_len = local_coeffs.len() * mpi_config.world_size();
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

            let mut chunk = 2 * local_coeffs.len();
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
