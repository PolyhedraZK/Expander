//! Two layer FFT and its inverse

use ark_std::log2;
use halo2curves::{
    ff::{Field, PrimeField},
    fft::{best_fft, FftGroup},
};

fn bitreverse(mut n: usize, l: usize) -> usize {
    let mut r = 0;
    for _ in 0..l {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}

#[inline]
fn deep_swap_chunks<F: Clone + Copy>(a: &mut [&mut [F]], rk: usize, k: usize) {
    let buf1 = (a[k]).to_vec();
    let buf2 = (a[rk]).to_vec();
    a[rk].copy_from_slice(&buf1);
    a[k].copy_from_slice(&buf2);
    // a[rk].iter_mut().zip(a[k].iter_mut()).for_each(|(a, b)| {
    //     let t = *a;
    //     *a = *b;
    //     *b = t});
}

// #[inline]
// fn swap_chunks<F>(a: &mut [F], log_n: u32) {

//     for k in 0..1<<log_n {
//         let rk = bitreverse(k, log_n as usize);
//         if k < rk {
//             a.swap(rk, k);
//         }
//     }

// }

fn assign_vec<F: Field>(a: &mut [F], b: &[F], n: usize) {
    assert!(a.len() == n);
    assert!(b.len() == n);
    a.iter_mut()
        .zip(b.iter())
        .take(n)
        .for_each(|(a, b)| *a = *b);
}

#[inline]
fn add_assign_vec<F: Field>(a: &mut [F], b: &[F], n: usize) {
    assert!(a.len() == n);
    assert!(b.len() == n);

    a.iter_mut()
        .zip(b.iter())
        .take(n)
        .for_each(|(a, b)| *a += b);
}

#[inline]
fn sub_assign_vec<F: Field>(a: &mut [F], b: &[F], n: usize) {
    assert!(a.len() == n);
    assert!(b.len() == n);
    a.iter_mut()
        .zip(b.iter())
        .take(n)
        .for_each(|(a, b)| *a -= b);
}

#[inline]
fn mul_assign_vec<F: Field>(a: &mut [F], b: &F, n: usize) {
    assert!(a.len() == n);
    a.iter_mut().take(n).for_each(|a| *a *= b);
}

// code copied from Halo2curves with adaption to vectors
//

/// Performs a radix-$2$ Fast-Fourier Transformation (FFT) on a vector of size
/// $n = 2^k$, when provided `log_n` = $k$ and an element of multiplicative
/// order $n$ called `omega` ($\omega$). The result is that the vector `a`, when
/// interpreted as the coefficients of a polynomial of degree $n - 1$, is
/// transformed into the evaluations of this polynomial at each of the $n$
/// distinct powers of $\omega$. This transformation is invertible by providing
/// $\omega^{-1}$ in place of $\omega$ and dividing each resulting field element
/// by $n$.
///
/// This will use multithreading if beneficial.
pub fn best_fft_vec<F: PrimeField>(a: &mut [F], omega: F, log_n: u32, log_m: u32) {
    let threads = rayon::current_num_threads();
    let log_threads = threads.ilog2();
    let mn = a.len();
    let m = 1 << log_m;
    let n = 1 << log_n;
    assert_eq!(mn, 1 << (log_n + log_m));

    // let mut a = (0..mn).map(|i| F::from(i as u64)).collect::<Vec<_>>();
    // println!("a: {:?}", a);
    // swap_chunks(a, log_n+log_m);
    // println!("a: {:?}", a);
    let mut a_vec_ptrs = a.chunks_exact_mut(n).collect::<Vec<_>>();

    // for a in a_vec_ptrs.iter_mut() {
    //     swap_chunks(a, log_n);
    // }

    for k in 0..m {
        let rk = bitreverse(k, log_m as usize);

        if k < rk {
            println!("k: {}, rk: {}", k, rk);
            for a in a_vec_ptrs.iter().enumerate() {
                println!("{}: {:?}", a.0, a.1);
            }

            deep_swap_chunks(&mut a_vec_ptrs, rk, k );

            // a_vec_ptrs.swap(rk, k);
            // swap_chunks(a_vec_ptrs[k], log_n);
            // swap_chunks(a_vec_ptrs[rk], log_n);

            for a in a_vec_ptrs.iter().enumerate() {
                println!("{}: {:?}", a.0, a.1);
            }

            println!();
        }
    }

    // precompute twiddle factors
    let twiddles: Vec<_> = (0..(m / 2))
        .scan(F::ONE, |w, _| {
            let tw = *w;
            *w *= &omega;
            Some(tw)
        })
        .collect();

    // if log_n <= log_threads {
    let mut chunk = 2_usize;
    let mut twiddle_chunk = m / 2;
    for _ in 0..log_m {
        a_vec_ptrs.chunks_mut(chunk).for_each(|coeffs| {
            let (left, right) = coeffs.split_at_mut(chunk / 2);

            // case when twiddle factor is one
            let (a, left) = left.split_at_mut(1);
            let (b, right) = right.split_at_mut(1);
            let t = b[0].to_vec();

            // b[0] = a[0];
            // a[0] += &t;
            // b[0] -= &t;
            assign_vec(b[0], a[0], n);
            add_assign_vec(a[0], &t, n);
            sub_assign_vec(b[0], &t, n);

            left.iter_mut()
                .zip(right.iter_mut())
                .enumerate()
                .for_each(|(i, (a, b))| {
                    let mut t = b.to_vec();

                    // t *= &twiddles[(i + 1) * twiddle_chunk];
                    // *b = *a;
                    // *a += &t;
                    // *b -= &t;

                    mul_assign_vec(&mut t, &twiddles[(i + 1) * twiddle_chunk], n);
                    assign_vec(b, a, n);
                    add_assign_vec(a, &t, n);
                    sub_assign_vec(b, &t, n);
                });
        });
        chunk *= 2;
        twiddle_chunk /= 2;
    }
    // // } else {
    // //     recursive_butterfly_arithmetic(a, n, 1, &twiddles)
    // // }
    // println!("before is correct: {:?}", a_vec_ptrs);

    // for k in 0..m {
    //     let rk = bitreverse(k, log_m as usize);

    //     if k < rk {
    //         println!("k: {}, rk: {}", k, rk);
    //         for a in a_vec_ptrs.iter().enumerate() {
    //             println!("{}: {:?}", a.0, a.1);
    //         }

    //         a_vec_ptrs.swap(rk, k);
    //         // swap_chunks(a_vec_ptrs[k], log_n);
    //         // swap_chunks(a_vec_ptrs[rk], log_n);

    //         for a in a_vec_ptrs.iter().enumerate() {
    //             println!("{}: {:?}", a.0, a.1);
    //         }

    //         println!();
    //     }
    // }

    // println!("after: {:?}", a_vec_ptrs);
}

// /// This perform recursive butterfly arithmetic
// pub fn recursive_butterfly_arithmetic<Scalar: Field, G: FftGroup<Scalar>>(
//     a: &mut [G],
//     n: usize,
//     twiddle_chunk: usize,
//     twiddles: &[Scalar],
// ) {
//     if n == 2 {
//         let t = a[1];
//         a[1] = a[0];
//         a[0] += &t;
//         a[1] -= &t;
//     } else {
//         let (left, right) = a.split_at_mut(n / 2);
//         rayon::join(
//             || recursive_butterfly_arithmetic(left, n / 2, twiddle_chunk * 2, twiddles),
//             || recursive_butterfly_arithmetic(right, n / 2, twiddle_chunk * 2, twiddles),
//         );

//         // case when twiddle factor is one
//         let (a, left) = left.split_at_mut(1);
//         let (b, right) = right.split_at_mut(1);
//         let t = b[0];
//         b[0] = a[0];
//         a[0] += &t;
//         b[0] -= &t;

//         left.iter_mut()
//             .zip(right.iter_mut())
//             .enumerate()
//             .for_each(|(i, (a, b))| {
//                 let mut t = *b;
//                 t *= &twiddles[(i + 1) * twiddle_chunk];
//                 *b = *a;
//                 *a += &t;
//                 *b -= &t;
//             });
//     }
// }

pub(crate) fn bi_fft_in_place<F: PrimeField>(coeffs: &mut [F], degree_n: usize, degree_m: usize) {
    // roots of unity for supported_n and supported_m
    let (omega_0, omega_1) = {
        let omega = F::ROOT_OF_UNITY;
        let omega_0 = omega.pow_vartime(&[(1 << F::S) / degree_n as u64]);
        let omega_1 = omega.pow_vartime(&[(1 << F::S) / degree_m as u64]);

        assert!(
            omega_0.pow_vartime(&[degree_n as u64]) == F::ONE,
            "omega_0 is not root of unity for supported_n"
        );
        assert!(
            omega_1.pow_vartime(&[degree_m as u64]) == F::ONE,
            "omega_1 is not root of unity for supported_m"
        );
        (omega_0, omega_1)
    };

    coeffs
        .chunks_exact_mut(degree_n)
        .for_each(|chunk| best_fft(chunk, omega_0, log2(degree_n)));

    println!("before: {:?}", coeffs);
    best_fft_vec(coeffs, omega_1, log2(degree_n), log2(degree_m));
    println!("after: {:?}", coeffs);
}

#[cfg(test)]
mod tests {
    use ark_std::test_rng;
    use halo2curves::bn256::Fr;

    use crate::BivariatePolynomial;

    use super::bi_fft_in_place;

    #[test]
    fn test_bi_fft() {
        {
            let n = 4;
            let m = 4;
            let poly = BivariatePolynomial::new(
                vec![
                    Fr::from(1u64),
                    Fr::from(2u64),
                    Fr::from(4u64),
                    Fr::from(8u64),
                    Fr::from(16u64),
                    Fr::from(32u64),
                    Fr::from(64u64),
                    Fr::from(128u64),
                    Fr::from(256u64),
                    Fr::from(128u64),
                    Fr::from(64u64),
                    Fr::from(32u64),
                    Fr::from(16u64),
                    Fr::from(8u64),
                    Fr::from(4u64),
                    Fr::from(2u64),
                ],
                n,
                m,
            );
            let mut poly_lag2 = poly.coefficients.clone();
            let poly_lag = poly.lagrange_coeffs();
            bi_fft_in_place(&mut poly_lag2, n, m);

            for (i, (a, b)) in poly_lag.iter().zip(poly_lag2.iter()).enumerate() {
                println!("{}: {:?} == {:?}", i, a, b);
            }

            println!("correct one {:?}", poly_lag);
            // println!();
            // println!("{:?}", poly_lag2);
            // println!();
            assert_eq!(poly_lag, poly_lag2);
        }

        let mut rng = test_rng();

        for m in [2, 4, 8, 16].iter() {
            for n in [2, 4, 8, 16].iter() {
                let poly = BivariatePolynomial::<Fr>::random(&mut rng, *n, *m);
                let mut poly_lag2 = poly.coefficients.clone();
                let poly_lag = poly.lagrange_coeffs();
                bi_fft_in_place(&mut poly_lag2, *n, *m);
                println!("m = {}, n = {}: {}", m, n, poly_lag == poly_lag2);
                assert_eq!(poly_lag, poly_lag2);
            }
        }
    }
}
