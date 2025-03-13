use crate::Field;

pub trait FFTField: Field {
    const TWO_ADICITY: usize;

    fn root_of_unity() -> Self;

    fn two_adic_generator(bits: usize) -> Self;

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
        let omega = Self::two_adic_generator(po2_mul_subgroup_bits);

        radix2_fft_single_threaded(poly, omega)
    }

    #[inline(always)]
    fn ifft_in_place(evals: &mut [Self]) {
        let po2_mul_subgroup_bits = evals.len().ilog2() as usize;
        let omega = Self::two_adic_generator(po2_mul_subgroup_bits);
        let omega_inv = omega.inv().unwrap();

        // TODO(HS) not good if the FFT size is larger than 2^32
        let n_inv = Self::from(evals.len() as u32).inv().unwrap();

        radix2_fft_single_threaded(evals, omega_inv);
        evals.iter_mut().for_each(|x| *x *= n_inv);
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

        let swap_to = bit_reverse(i, log_n);
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
        let mut stride = n >> 1;

        let mut res = vec![];

        for _ in 0..log_n {
            res.push((twiddle, stride));

            twiddle = twiddle * twiddle;
            stride >>= 1;
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

#[cfg(test)]
mod fft_test {
    use ark_std::test_rng;
    use halo2curves::bn256::Fr;

    use crate::{FFTField, Field};

    #[test]
    fn test_bn254_fft() {
        let mut rng = test_rng();

        (1..10).for_each(|bits| {
            let length = 1 << bits;

            let mut coeffs: Vec<_> = (0..length).map(|_| Fr::random_unsafe(&mut rng)).collect();
            let coeffs_cloned = coeffs.clone();

            Fr::fft_in_place(&mut coeffs);
            Fr::ifft_in_place(&mut coeffs);

            coeffs
                .iter()
                .zip(&coeffs_cloned)
                .for_each(|(a, b)| assert_eq!(a, b));
        });
    }
}

// NOTE(HS) acknowledge to https://github.com/EugeneGonzalez/bit_reverse
#[rustfmt::skip]
const U8_REVERSE_LOOKUP: [u8; 256] = [
    0,  128, 64, 192, 32, 160,  96, 224, 16, 144, 80, 208, 48, 176, 112, 240,
    8,  136, 72, 200, 40, 168, 104, 232, 24, 152, 88, 216, 56, 184, 120, 248,
    4,  132, 68, 196, 36, 164, 100, 228, 20, 148, 84, 212, 52, 180, 116, 244,
    12, 140, 76, 204, 44, 172, 108, 236, 28, 156, 92, 220, 60, 188, 124, 252,
    2,  130, 66, 194, 34, 162,  98, 226, 18, 146, 82, 210, 50, 178, 114, 242,
    10, 138, 74, 202, 42, 170, 106, 234, 26, 154, 90, 218, 58, 186, 122, 250,
    6,  134, 70, 198, 38, 166, 102, 230, 22, 150, 86, 214, 54, 182, 118, 246,
    14, 142, 78, 206, 46, 174, 110, 238, 30, 158, 94, 222, 62, 190, 126, 254,
    1,  129, 65, 193, 33, 161,  97, 225, 17, 145, 81, 209, 49, 177, 113, 241,
    9,  137, 73, 201, 41, 169, 105, 233, 25, 153, 89, 217, 57, 185, 121, 249,
    5,  133, 69, 197, 37, 165, 101, 229, 21, 149, 85, 213, 53, 181, 117, 245,
    13, 141, 77, 205, 45, 173, 109, 237, 29, 157, 93, 221, 61, 189, 125, 253,
    3,  131, 67, 195, 35, 163,  99, 227, 19, 147, 83, 211, 51, 179, 115, 243,
    11, 139, 75, 203, 43, 171, 107, 235, 27, 155, 91, 219, 59, 187, 123, 251,
    7,  135, 71, 199, 39, 167, 103, 231, 23, 151, 87, 215, 55, 183, 119, 247,
    15, 143, 79, 207, 47, 175, 111, 239, 31, 159, 95, 223, 63, 191, 127, 255
];

#[inline(always)]
fn bit_reverse_u8(a: u8) -> u8 {
    U8_REVERSE_LOOKUP[a as usize]
}

#[inline(always)]
fn bit_reverse_u16(a: u16) -> u16 {
    (U8_REVERSE_LOOKUP[a as u8 as usize] as u16) << 8
        | U8_REVERSE_LOOKUP[(a >> 8) as u8 as usize] as u16
}

#[inline(always)]
fn bit_reverse_u32(a: u32) -> u32 {
    (U8_REVERSE_LOOKUP[a as u8 as usize] as u32) << 24
        | (U8_REVERSE_LOOKUP[(a >> 8) as u8 as usize] as u32) << 16
        | (U8_REVERSE_LOOKUP[(a >> 16) as u8 as usize] as u32) << 8
        | (U8_REVERSE_LOOKUP[(a >> 24) as u8 as usize] as u32)
}

#[inline(always)]
fn bit_reverse_u64(a: u64) -> u64 {
    (U8_REVERSE_LOOKUP[a as u8 as usize] as u64) << 56
        | (U8_REVERSE_LOOKUP[(a >> 8) as u8 as usize] as u64) << 48
        | (U8_REVERSE_LOOKUP[(a >> 16) as u8 as usize] as u64) << 40
        | (U8_REVERSE_LOOKUP[(a >> 24) as u8 as usize] as u64) << 32
        | (U8_REVERSE_LOOKUP[(a >> 32) as u8 as usize] as u64) << 24
        | (U8_REVERSE_LOOKUP[(a >> 40) as u8 as usize] as u64) << 16
        | (U8_REVERSE_LOOKUP[(a >> 48) as u8 as usize] as u64) << 8
        | (U8_REVERSE_LOOKUP[(a >> 56) as u8 as usize] as u64)
}

#[inline(always)]
pub(crate) fn bit_reverse(mut n: usize, bit_width: usize) -> usize {
    let mut right_shift: usize = 0;

    if bit_width <= 8 {
        n = bit_reverse_u8(n as u8) as usize;
        right_shift = 8 - bit_width;
    } else if bit_width <= 16 {
        n = bit_reverse_u16(n as u16) as usize;
        right_shift = 16 - bit_width;
    } else if bit_width <= 32 {
        n = bit_reverse_u32(n as u32) as usize;
        right_shift = 32 - bit_width;
    } else if bit_width <= 64 {
        n = bit_reverse_u64(n as u64) as usize;
        right_shift = 64 - bit_width;
    }

    n >> right_shift
}

#[cfg(test)]
mod bit_reverse_test {
    use crate::bit_reverse;

    #[test]
    fn test_lut_bit_reverse() {
        (1..33).for_each(|width| {
            dbg!(width);
            (0..((1 << width) - 1))
                .for_each(|i| assert_eq!(bit_reverse(bit_reverse(i, width), width), i))
        })
    }
}
