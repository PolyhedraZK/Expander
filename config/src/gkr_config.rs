#[derive(Debug, Clone, PartialEq, Default)]
pub enum FiatShamirHashType {
    #[default]
    SHA256,
    Keccak256,
    Poseidon,
    Animoe,
    MIMC5, // Note: use MIMC5 for bn254 ONLY
}

/// Fiat Shamir hash type
const FIAT_SHAMIR_HASH: FiatShamirHashType;

/// Evaluate the circuit values at the challenge
#[inline]
fn eval_circuit_vals_at_challenge(
    evals: &[Self::SimdCircuitField],
    x: &[Self::ChallengeField],
    scratch: &mut [Self::Field],
) -> Self::Field {
    let timer = start_timer!(|| format!("eval mle with {} vars", x.len()));
    assert_eq!(1 << x.len(), evals.len());

    let ret = if x.is_empty() {
        Self::simd_circuit_field_into_field(&evals[0])
    } else {
        for i in 0..(evals.len() >> 1) {
            scratch[i] = Self::field_add_simd_circuit_field(
                &Self::simd_circuit_field_mul_challenge_field(
                    &(evals[i * 2 + 1] - evals[i * 2]),
                    &x[0],
                ),
                &evals[i * 2],
            );
        }

        let mut cur_eval_size = evals.len() >> 2;
        for r in x.iter().skip(1) {
            for i in 0..cur_eval_size {
                scratch[i] = scratch[i * 2] + (scratch[i * 2 + 1] - scratch[i * 2]).scale(r);
            }
            cur_eval_size >>= 1;
        }
        scratch[0]
    };
    end_timer!(timer);

    ret
}