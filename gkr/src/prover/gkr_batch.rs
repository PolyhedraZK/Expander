//! Batch GKR prover: prove N instances of the same circuit in ONE GKR proof.
//!
//! Takes N circuits (same structure, different witness values), runs one shared
//! sumcheck across all layers, and produces ONE proof with ONE evaluation point
//! suitable for ONE PCS opening.

use arith::SimdField;
use circuit::Circuit;
use gkr_engine::{
    ExpanderDualVarChallenge, ExpanderSingleVarChallenge, FieldEngine, Transcript,
};
use polynomials::MultiLinearPoly;
use sumcheck::{sumcheck_prove_gkr_layer_batch, ProverScratchPad};

/// Prove N circuit instances in one batch GKR.
///
/// All circuits must have the same structure (same layers, same gates).
/// Only input_vals and the resulting intermediate values may differ.
///
/// Returns (claimed_v, challenge) where:
/// - claimed_v is the batch-combined evaluation of the output layer
/// - challenge has r_mpi with log2(N) elements encoding the batch dimension
#[allow(clippy::type_complexity)]
pub fn gkr_prove_batch<F: FieldEngine>(
    circuits: &[Circuit<F>],
    scratch_pads: &mut [ProverScratchPad<F>],
    transcript: &mut impl Transcript,
) -> (F::ChallengeField, ExpanderDualVarChallenge<F>) {
    let n = circuits.len();
    assert_eq!(n, scratch_pads.len());
    assert!(n > 1 && n.is_power_of_two());

    let layer_num = circuits[0].layers.len();

    // Sample initial challenge with MPI dimension = N (the batch size).
    let mut challenge: ExpanderDualVarChallenge<F> =
        ExpanderSingleVarChallenge::sample_from_transcript(
            transcript,
            circuits[0].layers.last().unwrap().output_var_num,
            n, // world_size = N for batch
        )
        .into();

    // Compute batch-combined claimed_v from all instances' output values.
    let claimed_v = batch_eval_output_vals::<F>(circuits, &challenge.challenge_x());

    let mut alpha = None;
    for i in (0..layer_num).rev() {
        // Collect layer references from all circuits.
        let layers: Vec<&_> = circuits.iter().map(|c| &c.layers[i]).collect();

        sumcheck_prove_gkr_layer_batch(
            &layers,
            &mut challenge,
            alpha,
            transcript,
            scratch_pads,
            i == layer_num - 1,
        );

        if challenge.rz_1.is_some() {
            alpha = Some(transcript.generate_field_element::<F::ChallengeField>());
        } else {
            alpha = None;
        }
    }

    (claimed_v, challenge)
}

/// Evaluate the batch-combined output polynomial at the challenge point.
///
/// For each instance j, evaluates output_vals_j at (rz, r_simd), then
/// combines with eq(r_mpi, j) to get the batch-aggregated claimed value.
fn batch_eval_output_vals<F: FieldEngine>(
    circuits: &[Circuit<F>],
    challenge: &ExpanderSingleVarChallenge<F>,
) -> F::ChallengeField {
    let n = circuits.len();
    let simd_pack_size = <F::SimdCircuitField as SimdField>::PACK_SIZE;

    // Evaluate each instance's output at (rz, r_simd).
    let local_evals: Vec<F::ChallengeField> = circuits
        .iter()
        .map(|circuit| {
            let output_vals = &circuit.layers.last().unwrap().output_vals;

            // Evaluate multilinear polynomial at rz (over the circuit variables).
            let mut scratch = vec![F::Field::default(); output_vals.len()];
            let local_simd =
                F::eval_circuit_vals_at_challenge(output_vals, &challenge.rz, &mut scratch);

            // Unpack SIMD and evaluate at r_simd.
            let unpacked = local_simd.unpack();
            let mut simd_scratch = vec![F::ChallengeField::default(); simd_pack_size];
            MultiLinearPoly::evaluate_with_buffer(&unpacked, &challenge.r_simd, &mut simd_scratch)
        })
        .collect();

    // Combine with eq(r_mpi, j) for j = 0..N-1.
    let mut mpi_scratch = vec![F::ChallengeField::default(); n];
    MultiLinearPoly::evaluate_with_buffer(&local_evals, &challenge.r_mpi, &mut mpi_scratch)
}
