use halo2curves::ff::{Field, PrimeField};

pub fn primitive_root_of_unity<F: PrimeField>(group_size: usize) -> F {
    let omega = F::ROOT_OF_UNITY;
    let omega = omega.pow_vartime([(1 << F::S) / group_size as u64]);
    assert!(
        omega.pow_vartime([group_size as u64]) == F::ONE,
        "omega_0 is not root of unity for supported_n"
    );
    omega
}

pub(crate) fn powers_of_field_elements<F: Field>(x: &F, n: usize) -> Vec<F> {
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
