use crate::Field;

pub trait FFTField: Field {
    const TWO_ADICITY: usize;

    fn root_of_unity() -> Self;

    fn primitive_po2_root_of_unity(bits: usize) -> Self;

    #[inline(always)]
    fn fft(poly: &[Self]) -> Vec<Self> {
        let mut coeffs = poly.to_vec();
        Self::fft_in_place(&mut coeffs);
        coeffs
    }

    #[inline(always)]
    fn ifft(evals: &[Self]) -> Vec<Self> {
        let mut coeffs = evals.to_vec();
        Self::ifft_in_place(&mut coeffs);
        coeffs
    }

    #[inline(always)]
    fn fft_in_place(poly: &mut [Self]) {
        let po2_mul_subgroup_bits = poly.len().ilog2() as usize;
        let omega = Self::primitive_po2_root_of_unity(po2_mul_subgroup_bits);

        radix2_fft_single_threaded(poly, omega)
    }

    #[inline(always)]
    fn ifft_in_place(evals: &mut [Self]) {
        let po2_mul_subgroup_bits = evals.len().ilog2() as usize;
        let omega = Self::primitive_po2_root_of_unity(po2_mul_subgroup_bits);
        let omega_inv = omega.inv().unwrap();

        evals[1..].reverse();
        radix2_fft_single_threaded(evals, omega_inv)
    }
}

#[inline(always)]
fn bit_reverse_swap<F: Copy>(elems: &mut [F]) {
    // NOTE(HS) we are assuming that this method is only used in FFT,
    // then the elems slice here is assumed to be of length power of 2.

    let mut swap_count: usize = 0;
    let n = elems.len();
    let log_n = n.ilog2() as usize;
    let swap_threshold = n >> 1;

    for i in 0..n {
        // NOTE(HS) swap number should be exactly half of the elems
        if swap_count >= swap_threshold {
            break;
        }

        // TODO(HS) accelerate shit out of the bit swap here
        let swap_to = bitreverse(i, log_n);
        if i < swap_to {
            // NOTE(HS) the invariant here is bit swap won't exceed the range,
            // so we choose to use unchecked to short wire the range check,
            // s.t., less instructions
            unsafe { elems.swap_unchecked(swap_to, i) }
            swap_count += 1;
        }
    }
}

#[inline(always)]
pub fn radix2_fft_single_threaded<F: FFTField>(coeffs: &mut [F], omega: F) {
    assert!(coeffs.len().is_power_of_two());

    bit_reverse_swap(coeffs);

    let n = coeffs.len();
    let log_n = n.ilog2() as usize;

    let twiddles_and_strides = {
        let mut twiddle = omega;
        let mut stride = 1usize;

        let mut res = vec![];

        for _ in 0..log_n {
            res.push((twiddle, stride));

            twiddle = twiddle * twiddle;
            stride *= 2;
        }

        res.reverse();
        res
    };

    twiddles_and_strides.iter().for_each(|(twiddle, stride)| {
        for left_most_index in (0..n).step_by(2 * stride) {
            let mut p = F::ONE;

            for i in 0..*stride {
                let left = coeffs[left_most_index + i];
                let right = coeffs[left_most_index + stride + i];

                let t = p * right;

                coeffs[left_most_index + stride + i] = left;

                coeffs[left_most_index + i] += t;
                coeffs[left_most_index + stride + i] -= t;

                p *= twiddle;
            }
        }
    });
}

#[inline]
fn bitreverse(mut n: usize, l: usize) -> usize {
    let mut r = 0;
    for _ in 0..l {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}
