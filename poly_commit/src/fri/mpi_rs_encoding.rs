use arith::FFTField;
use gkr_engine::MPIEngine;
use itertools::izip;

use crate::utils::mpi_matrix_transpose;

#[allow(unused)]
#[inline(always)]
pub(crate) fn mpi_reed_solomon_encoding<F: FFTField>(
    mpi_config: &impl MPIEngine,
    local_coeffs: &[F],
    code_rate_log2_bits: usize,
) -> Vec<F> {
    assert!(local_coeffs.len().is_power_of_two());

    let (col_size, generator): (usize, F) = {
        let hypercube_bits = local_coeffs.len().ilog2() + mpi_config.world_size().ilog2();
        let codeword_bits = hypercube_bits as usize + code_rate_log2_bits;

        let mut col_size = 1;
        if codeword_bits > F::TWO_ADICITY {
            col_size <<= (codeword_bits - F::TWO_ADICITY);
        }

        let full_codeword_bits = std::cmp::min(codeword_bits, F::TWO_ADICITY);
        let bits = full_codeword_bits - code_rate_log2_bits;
        let generator = F::two_adic_generator(bits);

        (col_size, generator)
    };

    let mut codeword: Vec<F> = local_coeffs.to_vec();

    mpi_matrix_transpose(mpi_config, &mut codeword, col_size);

    {
        let local_codeword_len = codeword.len();
        codeword.resize(local_codeword_len << code_rate_log2_bits, F::ZERO);

        let coset_size = 1 << code_rate_log2_bits;
        for i in (0..local_codeword_len).rev() {
            let temp = codeword[i];
            codeword[i * coset_size..(i + 1) * coset_size].fill(temp);
        }
    }

    radix2_fft_local(&mut codeword, generator, col_size);

    // NOTE(HS) bit inverse and FFT for MPI world variables
    {
        let mut root_codeword = {
            let global_len = local_coeffs.len() * mpi_config.world_size();
            vec![F::ZERO; if mpi_config.is_root() { global_len } else { 0 }]
        };

        mpi_config.gather_vec(&codeword, &mut root_codeword);

        let num_world_bits = (mpi_config.world_size() - 1).count_ones();

        // NOTE(HS) only the root process runs the remaining FFT
        if mpi_config.is_root() {
            // TODO(HS) root remaining generator
            // TODO(HS) run the remaining FFT
        }

        // NOTE finally scatter back into own world
        mpi_config.scatter_vec(&root_codeword, &mut codeword);
    }

    codeword
}

#[inline(always)]
fn radix2_fft_local<F: FFTField>(coeffs: &mut [F], omega: F, col_size: usize) {
    assert!(coeffs.len().is_power_of_two());

    let n = coeffs.len() / col_size;
    let log_n = n.ilog2() as usize;

    // precompute twiddle factors
    let twiddles: Vec<_> = (0..(n / 2))
        .scan(F::ONE, |w, _| {
            let tw = *w;
            *w *= &omega;
            Some(tw)
        })
        .collect();

    let mut chunk = 2_usize * col_size;
    let mut twiddle_chunk = n / 2;
    for _ in 0..log_n {
        coeffs.chunks_mut(chunk).for_each(|coeffs| {
            let (left, right) = coeffs.split_at_mut(chunk / 2);

            // case when twiddle factor is one
            let (a, left) = left.split_at_mut(col_size);
            let (b, right) = right.split_at_mut(col_size);

            izip!(a, b).for_each(|(a_i, b_i)| {
                let t = *b_i;
                *b_i = *a_i;
                *a_i += &t;
                *b_i -= &t;
            });

            izip!(left.chunks_mut(col_size), right.chunks_mut(col_size))
                .enumerate()
                .for_each(|(i, (left_chunk, right_chunk))| {
                    izip!(left_chunk, right_chunk).for_each(|(a, b)| {
                        let mut t = *b;
                        t *= &twiddles[(i + 1) * twiddle_chunk];
                        *b = *a;
                        *a += &t;
                        *b -= &t;
                    });
                });
        });
        chunk *= 2;
        twiddle_chunk /= 2;
    }
}
