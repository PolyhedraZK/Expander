use std::io::Cursor;

use arith::{ExtensionField, Field, Fr, SimdField};
use ark_std::test_rng;
use gkr_engine::{BN254Config, FieldEngine, MPIConfig, MPIEngine, Transcript};
use gkr_hashers::Keccak256hasher;
use polynomials::MultiLinearPoly;
use sumcheck::{gkr_prove, gkr_verify};
use transcript::BytesHashTranscript;

use crate::{
    orion::{
        code_switching::{
            code_switching_gkr_circuit, prepare_code_switching_gkr_prover_mem,
            prepare_code_switching_inputs, CODE_SWITCHING_WORLD_SIZE,
        },
        linear_code::OrionCode,
    },
    ORION_CODE_PARAMETER_INSTANCE,
};

fn test_orion_code_switch_gkr_helper<F, C>(num_vars: usize, mpi_config: &MPIConfig)
where
    F: Field + ExtensionField + SimdField,
    C: FieldEngine<CircuitField = F, ChallengeField = F, SimdCircuitField = F>,
{
    const PROXIMITY_REPETITIONS: usize = 2;

    assert_eq!(C::SimdCircuitField::PACK_SIZE, 1);

    let mut rng = test_rng();

    let msg_size = 1 << num_vars;
    let encoder = OrionCode::new(ORION_CODE_PARAMETER_INSTANCE, msg_size, &mut rng);

    let challenge_point: Vec<_> = (0..num_vars)
        .map(|_| C::ChallengeField::random_unsafe(&mut rng))
        .collect();

    let mut layered_circuit =
        code_switching_gkr_circuit::<F, C>(&encoder, &challenge_point, PROXIMITY_REPETITIONS);

    let evals_poly = MultiLinearPoly::<C::SimdCircuitField>::random(num_vars, &mut rng);
    let prox_poly0 = MultiLinearPoly::<C::SimdCircuitField>::random(num_vars, &mut rng);
    let prox_poly1 = MultiLinearPoly::<C::SimdCircuitField>::random(num_vars, &mut rng);

    let input_coeffs = prepare_code_switching_inputs(
        &evals_poly.coeffs,
        &[prox_poly0.coeffs.clone(), prox_poly1.coeffs.clone()],
    );
    layered_circuit.layers[0].input_vals = input_coeffs.clone();
    layered_circuit.evaluate();

    {
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

    {
        let mut sp = prepare_code_switching_gkr_prover_mem(&layered_circuit);
        let mut fs_transcript_prover = BytesHashTranscript::<F, Keccak256hasher>::new();
        let mut fs_transcript_verifier = fs_transcript_prover.clone();

        let (claimed_v, challenge_prover) = gkr_prove(
            &layered_circuit,
            &mut sp,
            &mut fs_transcript_prover,
            mpi_config,
        );
        assert!(challenge_prover.rz_1.is_none());

        let proof_bytes = fs_transcript_prover.finalize_and_get_proof();
        let mut proof_reader = Cursor::new(&proof_bytes.bytes);
        let (verified, challenge_verifier, claimed_v0_verifier, claimed_v1_verifier) = gkr_verify(
            CODE_SWITCHING_WORLD_SIZE,
            &layered_circuit,
            &[],
            &claimed_v,
            &mut fs_transcript_verifier,
            &mut proof_reader,
        );
        assert!(challenge_verifier.rz_1.is_none());

        assert!(claimed_v1_verifier.is_none());
        assert_eq!(&challenge_verifier.rz_0, &challenge_prover.rz_0);
        assert!(verified);

        let input_mle = MultiLinearPoly::new(input_coeffs);
        let expected_final_claim_v = input_mle.evaluate_jolt(&challenge_verifier.rz_0);

        assert_eq!(claimed_v0_verifier, expected_final_claim_v);
    }
}

#[test]
fn test_orion_code_switch_gkr() {
    let mpi_config = MPIConfig::prover_new();
    assert_eq!(mpi_config.world_size(), CODE_SWITCHING_WORLD_SIZE);

    test_orion_code_switch_gkr_helper::<Fr, BN254Config>(15, &mpi_config);

    MPIConfig::finalize()
}
