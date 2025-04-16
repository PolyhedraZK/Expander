use arith::{Field, Fr};
use ark_std::test_rng;
use gkr_engine::{BN254Config, FieldEngine};
use polynomials::MultiLinearPoly;

use crate::{
    code_switching_gkr_circuit, orion::linear_code::OrionCode, ORION_CODE_PARAMETER_INSTANCE,
};

fn test_orion_code_switch_circuit_evaluate_helper<F: Field, C: FieldEngine>(num_vars: usize)
where
    F: Field,
    C: FieldEngine<CircuitField = F, ChallengeField = F, SimdCircuitField = F>,
{
    const PROXIMITY_REPETITIONS: usize = 2;

    let mut rng = test_rng();

    let msg_size = 1 << num_vars;
    let encoder = OrionCode::new(ORION_CODE_PARAMETER_INSTANCE, msg_size, &mut rng);

    let evals_poly = MultiLinearPoly::<C::SimdCircuitField>::random(num_vars, &mut rng);
    let prox_poly0 = MultiLinearPoly::<C::SimdCircuitField>::random(num_vars, &mut rng);
    let prox_poly1 = MultiLinearPoly::<C::SimdCircuitField>::random(num_vars, &mut rng);

    let input_coeffs = {
        let mut buf = evals_poly.coeffs.clone();
        buf.resize(msg_size * 2, F::ZERO);
        buf.extend_from_slice(&prox_poly0.coeffs);
        buf.extend_from_slice(&prox_poly1.coeffs);
        buf
    };

    let challenge_point: Vec<_> = (0..num_vars)
        .map(|_| C::ChallengeField::random_unsafe(&mut rng))
        .collect();

    let mut layered_circuit =
        code_switching_gkr_circuit::<F, C>(&encoder, &challenge_point, PROXIMITY_REPETITIONS);

    layered_circuit.layers[0].input_vals = input_coeffs.clone();

    layered_circuit.evaluate();

    let expected = {
        let mut evals_encoded = encoder.encode(&evals_poly.coeffs).unwrap();
        evals_encoded.resize(msg_size * 2, F::ZERO);

        let mut prox0_encoded = encoder.encode(&prox_poly0.coeffs).unwrap();
        prox0_encoded.resize(msg_size * 2, F::ZERO);

        let mut prox1_encoded = encoder.encode(&prox_poly1.coeffs).unwrap();
        prox1_encoded.resize(msg_size * 2, F::ZERO);

        let evaluation = evals_poly.evaluate_jolt(&challenge_point);
        let mut buffer = vec![evaluation];
        buffer.resize(msg_size * 2, F::ZERO);

        buffer.extend(evals_encoded);
        buffer.extend(prox0_encoded);
        buffer.extend(prox1_encoded);

        buffer
    };

    assert_eq!(expected, layered_circuit.layers.last().unwrap().output_vals);
}

#[test]
fn test_orion_code_switch_circuit_evaluate() {
    test_orion_code_switch_circuit_evaluate_helper::<Fr, BN254Config>(15);
}
