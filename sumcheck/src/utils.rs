use arith::{ExtensionField, Field, SimdField};
use gkr_engine::{MPIEngine, Transcript};

#[inline(always)]
/// Input
/// - a SIMD field, denoted by p := [p0, ... pn]
/// - a vector of coefficients, denoted by coef := [c0, ... cn]
///
/// Output
/// - p0 * c0 + ... + pn * cn
pub fn unpack_and_combine<F: SimdField>(p: &F, coef: &[F::Scalar]) -> F::Scalar {
    if coef.len() >= F::PACK_SIZE {
        let coef_packed = F::pack(&coef[..F::PACK_SIZE]);
        return (coef_packed * p).horizontal_sum();
    }

    let p_unpacked = p.unpack();
    p_unpacked
        .into_iter()
        .zip(coef)
        .map(|(p_i, coef_i)| p_i * coef_i)
        .sum()
}

/// Transcript IO between sumcheck steps
#[inline]
pub fn transcript_io<F, T>(mpi_config: &impl MPIEngine, ps: &[F], transcript: &mut T) -> F
where
    F: ExtensionField,
    T: Transcript,
{
    // 3 for x, y; 4 for simd var; 7 for pow5, 9 for pow7
    assert!(
        ps.len() == 3 || ps.len() == 4 || ps.len() == 7 || ps.len() == 9,
        "Unexpected polynomial size"
    );
    for p in ps {
        transcript.append_field_element(p);
    }
    let mut r = transcript.generate_field_element::<F>();
    mpi_config.root_broadcast_f(&mut r);
    r
}

// Given a vector of field elements {v_i}, compute the vector {v_i^(-1)}
pub fn batch_inversion<F: Field>(v: &mut [F]) {
    batch_inversion_and_mul(v, &F::one());
}

// Given a vector of field elements {v_i}, compute the vector {coeff * v_i^(-1)}
pub fn batch_inversion_and_mul<F: Field>(v: &mut [F], coeff: &F) {
    serial_batch_inversion_and_mul(v, coeff);
}

/// Given a vector of field elements {v_i}, compute the vector {coeff * v_i^(-1)}.
/// This method is explicitly single-threaded.
fn serial_batch_inversion_and_mul<F: Field>(v: &mut [F], coeff: &F) {
    // Montgomery’s Trick and Fast Implementation of Masked AES
    // Genelle, Prouff and Quisquater
    // Section 3.2
    // but with an optimization to multiply every element in the returned vector by
    // coeff

    // First pass: compute [a, ab, abc, ...]
    let mut prod = Vec::with_capacity(v.len());
    let mut tmp = F::one();
    for f in v.iter().filter(|f| !f.is_zero()) {
        tmp.mul_assign(f);
        prod.push(tmp);
    }

    // Invert `tmp`.
    tmp = tmp.inv().unwrap(); // Guaranteed to be nonzero.

    // Multiply product by coeff, so all inverses will be scaled by coeff
    tmp *= coeff;

    // Second pass: iterate backwards to compute inverses
    for (f, s) in v.iter_mut()
        // Backwards
        .rev()
        // Ignore normalized elements
        .filter(|f| !f.is_zero())
        // Backwards, skip last element, fill in one for last term.
        .zip(prod.into_iter().rev().skip(1).chain(Some(F::one())))
    {
        // tmp := tmp * f; f := tmp * s = 1/f
        let new_tmp = tmp * *f;
        *f = tmp * &s;
        tmp = new_tmp;
    }
}
