use arith::FieldForECC;

const fn compile_time_gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

pub(crate) const fn compile_time_sbox_alpha<F: FieldForECC>() -> usize {
    let modulus = F::MODULUS.as_usize();

    let mut alpha: usize = 5;
    while compile_time_gcd(alpha, modulus) != 1 {
        alpha += 2
    }
    alpha
}
