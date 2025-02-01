use std::{iter::Sum, ops::Mul};

use halo2curves::ff::{Field, PrimeField};
use itertools::izip;

pub fn primitive_root_of_unity<F: PrimeField>(group_size: usize) -> F {
    let omega = F::ROOT_OF_UNITY;
    let omega = omega.pow_vartime([(1 << F::S) / group_size as u64]);
    assert!(
        omega.pow_vartime([group_size as u64]) == F::ONE,
        "omega_0 is not root of unity for supported_n"
    );
    omega
}

#[inline(always)]
pub(crate) fn powers_series<F: Field>(x: &F, n: usize) -> Vec<F> {
    let mut powers = vec![F::ONE];
    let mut cur = *x;
    for _ in 0..n - 1 {
        powers.push(cur);
        cur *= x;
    }
    powers
}

/// Given a univariate polynomial of coefficient form f(X) = c0 + c1 X + ... + cn X^n
/// and perform the division f(X) / (X - \alpha).
#[inline(always)]
pub(crate) fn univariate_degree_one_quotient<F: Field>(coeffs: &[F], alpha: F) -> (Vec<F>, F) {
    let mut div_coeffs = coeffs.to_vec();

    for i in (1..coeffs.len()).rev() {
        // c X^n = c X^n - c \alpha X^(n - 1) + c \alpha X^(n - 1)
        //       = c (X - \alpha) X^(n - 1) + c \alpha X^(n - 1)

        let remainder = div_coeffs[i] * alpha;
        div_coeffs[i - 1] += remainder;
    }

    let final_remainder = div_coeffs[0];
    let mut final_div_coeffs = div_coeffs[1..].to_owned();
    final_div_coeffs.resize(coeffs.len(), F::ZERO);

    (final_div_coeffs, final_remainder)
}

#[inline(always)]
pub(crate) fn univariate_roots_quotient<F: Field>(mut coeffs: Vec<F>, roots: &[F]) -> Vec<F> {
    roots.iter().enumerate().for_each(|(ith_root, r)| {
        for i in ((1 + ith_root)..coeffs.len()).rev() {
            // c X^n = c X^n - c \alpha X^(n - 1) + c \alpha X^(n - 1)
            //       = c (X - \alpha) X^(n - 1) + c \alpha X^(n - 1)

            let remainder = coeffs[i] * r;
            coeffs[i - 1] += remainder;
        }

        assert_eq!(coeffs[ith_root], F::ZERO);
    });

    coeffs[roots.len()..].to_owned()
}

#[inline(always)]
pub(crate) fn univariate_evaluate<F: Mul<F1, Output = F> + Sum + Copy, F1: Field>(
    coeffs: &[F],
    power_series: &[F1],
) -> F {
    izip!(coeffs, power_series).map(|(c, p)| *c * *p).sum()
}

pub(crate) fn even_odd_coeffs_separate<F: Field>(coeffs: &[F]) -> (Vec<F>, Vec<F>) {
    assert!(coeffs.len().is_power_of_two());

    let mut even = Vec::with_capacity(coeffs.len() >> 1);
    let mut odd = Vec::with_capacity(coeffs.len() >> 1);

    coeffs.chunks(2).for_each(|eo: &[F]| {
        even.push(eo[0]);
        odd.push(eo[1]);
    });

    (even, odd)
}

#[inline(always)]
pub(crate) fn polynomial_add<F: Field>(coeffs: &mut [F], weight: F, another_coeffs: &[F]) {
    assert!(coeffs.len() >= another_coeffs.len());

    izip!(coeffs, another_coeffs).for_each(|(c, a)| *c += weight * *a);
}
