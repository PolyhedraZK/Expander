//! Batch sumcheck: prove N instances of the same circuit layer in ONE shared sumcheck.
//!
//! Instead of N separate GKR proofs (each with full sumcheck + PCS opening), this
//! aggregates N instances into a single sumcheck using the MPI variable dimension.
//! All instances share ONE transcript, produce ONE set of challenges, and reduce
//! PCS cost from O(N^2) to O(N).

use arith::{Field, SimdField};
use circuit::CircuitLayer;
use gkr_engine::{ExpanderDualVarChallenge, FieldEngine, Transcript};
use polynomials::EqPolynomial;
use rayon::prelude::*;

use crate::{
    prover_helper::{
        product_gate::SumcheckProductGateHelper, simd_gate::SumcheckSimdProdGateHelper,
    },
    unpack_and_combine, ProverScratchPad, SUMCHECK_GKR_DEGREE, SUMCHECK_GKR_SIMD_MPI_DEGREE,
};

/// Transcript I/O for batch sumcheck (no MPI broadcast needed).
#[inline]
fn transcript_io_local<F: arith::ExtensionField, T: Transcript>(
    evals: &[F],
    transcript: &mut T,
) -> F {
    for p in evals {
        transcript.append_field_element(p);
    }
    transcript.generate_field_element::<F>()
}

/// Prepare hg_evals, eq_evals_at_rz0, gate_exists for one instance's x-phase.
/// Mirrors SumcheckGkrVanillaHelper::prepare_x_vals.
fn prepare_x_vals_for_instance<F: FieldEngine>(
    layer: &CircuitLayer<F>,
    challenge: &ExpanderDualVarChallenge<F>,
    alpha: Option<F::ChallengeField>,
    sp: &mut ProverScratchPad<F>,
    is_output_layer: bool,
) {
    let vals = &layer.input_vals;
    let eq_evals_at_rz0 = &mut sp.eq_evals_at_rz0;
    let gate_exists = &mut sp.gate_exists;
    let hg_vals = &mut sp.hg_evals;

    unsafe {
        std::ptr::write_bytes(hg_vals.as_mut_ptr(), 0, vals.len());
        std::ptr::write_bytes(gate_exists.as_mut_ptr(), 0, vals.len());
    }

    assert_eq!(challenge.rz_1.is_none(), alpha.is_none());

    if is_output_layer || challenge.rz_1.is_none() {
        EqPolynomial::<F::ChallengeField>::eq_eval_at(
            &challenge.rz_0,
            &F::ChallengeField::ONE,
            eq_evals_at_rz0,
            &mut sp.eq_evals_first_half,
            &mut sp.eq_evals_second_half,
        );
    } else {
        let alpha_val = alpha.unwrap();
        // eq_evals_at_rx holds eq(prev_rx, .) from the previous layer's y-phase.
        let eq_evals_at_rx_prev: Vec<F::ChallengeField> =
            sp.eq_evals_at_rx[..1 << challenge.rz_0.len()].to_vec();
        EqPolynomial::<F::ChallengeField>::eq_eval_at(
            challenge.rz_1.as_ref().unwrap(),
            &alpha_val,
            eq_evals_at_rz0,
            &mut sp.eq_evals_first_half,
            &mut sp.eq_evals_second_half,
        );
        for i in 0..(1 << challenge.rz_0.len()) {
            eq_evals_at_rz0[i] += eq_evals_at_rx_prev[i];
        }
    }

    for g in layer.mul.iter() {
        let r = eq_evals_at_rz0[g.o_id] * g.coef;
        hg_vals[g.i_ids[0]] += r * vals[g.i_ids[1]];
        gate_exists[g.i_ids[0]] = true;
    }

    for g in layer.add.iter() {
        hg_vals[g.i_ids[0]] += F::Field::from(eq_evals_at_rz0[g.o_id] * g.coef);
        gate_exists[g.i_ids[0]] = true;
    }
}

/// Prepare y-phase state for one instance.
/// Mirrors SumcheckGkrVanillaHelper::prepare_y_vals, adapted for batch mode:
/// - phase2_coef uses the BATCH r_mpi dimensions
/// - eq_evals_at_r_mpi0 is NOT recomputed (stays [1.0] for single-instance use;
///   batch aggregation happens externally)
#[allow(clippy::too_many_arguments)]
fn prepare_y_vals_for_instance<F: FieldEngine>(
    layer: &CircuitLayer<F>,
    rx: &[F::ChallengeField],
    r_simd_var: &[F::ChallengeField],
    r_mpi_var: &[F::ChallengeField],
    challenge: &ExpanderDualVarChallenge<F>,
    sp: &mut ProverScratchPad<F>,
    vx_claim: F::ChallengeField,
) {
    // phase2_coef bridges x-phase and y-phase claims.
    // Uses BATCH r_mpi from challenge and current layer's r_mpi_var.
    sp.phase2_coef = EqPolynomial::<F::ChallengeField>::eq_vec(&challenge.r_mpi, r_mpi_var)
        * sp.eq_evals_at_r_simd0[0]
        * vx_claim;

    // Recompute eq polynomials for the y-phase evaluation.
    // NOTE: eq_evals_at_r_mpi0 is NOT recomputed here — it stays as [1.0].
    // Batch MPI aggregation is done externally with batch_eq_y coefficients.
    EqPolynomial::<F::ChallengeField>::eq_eval_at(
        rx,
        &F::ChallengeField::ONE,
        &mut sp.eq_evals_at_rx,
        &mut sp.eq_evals_first_half,
        &mut sp.eq_evals_second_half,
    );
    EqPolynomial::<F::ChallengeField>::eq_eval_at(
        r_simd_var,
        &F::ChallengeField::ONE,
        &mut sp.eq_evals_at_r_simd0,
        &mut sp.eq_evals_first_half,
        &mut sp.eq_evals_second_half,
    );

    let eq_evals_at_rz0 = &sp.eq_evals_at_rz0;
    let eq_evals_at_rx = &sp.eq_evals_at_rx;
    let gate_exists = &mut sp.gate_exists;
    let hg_vals = &mut sp.hg_evals;
    let fill_len = 1 << rx.len();

    unsafe {
        std::ptr::write_bytes(hg_vals.as_mut_ptr(), 0, fill_len);
        std::ptr::write_bytes(gate_exists.as_mut_ptr(), 0, fill_len);
    }

    for g in layer.mul.iter() {
        hg_vals[g.i_ids[1]] +=
            F::Field::from(eq_evals_at_rz0[g.o_id] * eq_evals_at_rx[g.i_ids[0]] * g.coef);
        gate_exists[g.i_ids[1]] = true;
    }
}

/// Batch N instances of the same circuit layer in ONE shared sumcheck.
///
/// All instances must have the same circuit structure (same gates, topology).
/// Only `input_vals` differ. The batch shares ONE transcript and produces
/// ONE set of challenges including `r_mpi` over the N instances.
///
/// The challenge.r_mpi must have log2(N) elements on entry (from the previous
/// layer or initial sampling). On exit, challenge is updated with new
/// (rx, ry, r_simd, r_mpi).
pub fn sumcheck_prove_gkr_layer_batch<F: FieldEngine, T: Transcript>(
    layers: &[&CircuitLayer<F>],
    challenge: &mut ExpanderDualVarChallenge<F>,
    alpha: Option<F::ChallengeField>,
    transcript: &mut T,
    scratch_pads: &mut [ProverScratchPad<F>],
    is_output_layer: bool,
) -> (F::ChallengeField, Option<F::ChallengeField>) {
    let n = layers.len();
    assert_eq!(n, scratch_pads.len());
    assert!(n > 1 && n.is_power_of_two());

    let input_var_num = layers[0].input_var_num;
    let simd_var_num = <F::SimdCircuitField as SimdField>::PACK_SIZE.trailing_zeros() as usize;
    let mpi_var_num = n.trailing_zeros() as usize;

    // ---- Compute batch eq coefficients from challenge.r_mpi ----
    assert_eq!(challenge.r_mpi.len(), mpi_var_num);
    let batch_eq_coeffs = EqPolynomial::<F::ChallengeField>::build_eq_x_r(&challenge.r_mpi);

    // ---- Prepare SIMD eq polynomial (shared; compute once, copy to all) ----
    if is_output_layer || alpha.is_none() {
        let sp0 = &mut scratch_pads[0];
        EqPolynomial::<F::ChallengeField>::eq_eval_at(
            &challenge.r_simd,
            &F::ChallengeField::ONE,
            &mut sp0.eq_evals_at_r_simd0,
            &mut sp0.eq_evals_first_half,
            &mut sp0.eq_evals_second_half,
        );
        let shared: Vec<F::ChallengeField> = sp0.eq_evals_at_r_simd0.clone();
        for sp in scratch_pads[1..].iter_mut() {
            sp.eq_evals_at_r_simd0.copy_from_slice(&shared);
        }
    }

    // Each instance uses single-process MPI internally.
    for sp in scratch_pads.iter_mut() {
        sp.eq_evals_at_r_mpi0 = vec![F::ChallengeField::ONE];
    }

    // ======== Phase 1: x-variable sumcheck ========
    let mut xy_helpers: Vec<SumcheckProductGateHelper> = (0..n)
        .map(|_| SumcheckProductGateHelper::new(input_var_num))
        .collect();

    scratch_pads
        .par_iter_mut()
        .zip(layers.par_iter())
        .for_each(|(sp, layer)| {
            prepare_x_vals_for_instance(layer, challenge, alpha, sp, is_output_layer);
        });

    let mut all_rx = Vec::with_capacity(input_var_num);
    for i_var in 0..input_var_num {
        let agg = xy_helpers
            .par_iter_mut()
            .zip(scratch_pads.par_iter())
            .zip(layers.par_iter())
            .zip(batch_eq_coeffs.par_iter())
            .map(|(((helper, sp), layer), &eq_coeff)| {
                let local_simd = helper.poly_eval_at::<F>(
                    i_var,
                    SUMCHECK_GKR_DEGREE,
                    &sp.v_evals,
                    &sp.hg_evals,
                    &layer.input_vals,
                    &sp.gate_exists,
                );
                let mut local_agg = [F::ChallengeField::ZERO; 3];
                for k in 0..3 {
                    local_agg[k] =
                        eq_coeff * unpack_and_combine(&local_simd[k], &sp.eq_evals_at_r_simd0);
                }
                local_agg
            })
            .reduce(
                || [F::ChallengeField::ZERO; 3],
                |mut a, b| {
                    for k in 0..3 {
                        a[k] += b[k];
                    }
                    a
                },
            );
        let r = transcript_io_local::<F::ChallengeField, T>(&agg, transcript);
        xy_helpers
            .par_iter_mut()
            .zip(scratch_pads.par_iter_mut())
            .zip(layers.par_iter())
            .for_each(|((helper, sp), layer)| {
                helper.receive_challenge::<F>(
                    i_var,
                    r,
                    &mut sp.v_evals,
                    &mut sp.hg_evals,
                    &layer.input_vals,
                    &mut sp.gate_exists,
                );
            });
        all_rx.push(r);
    }

    // ======== Phase 2: SIMD variable sumcheck ========
    scratch_pads.par_iter_mut().for_each(|sp| {
        sp.simd_var_v_evals = sp.v_evals[0].unpack();
        sp.simd_var_hg_evals = sp.hg_evals[0].unpack();
    });

    let mut simd_helpers: Vec<SumcheckSimdProdGateHelper<F>> = (0..n)
        .map(|_| SumcheckSimdProdGateHelper::new(simd_var_num))
        .collect();

    let mut all_r_simd = Vec::with_capacity(simd_var_num);
    for i_var in 0..simd_var_num {
        let agg = simd_helpers
            .par_iter_mut()
            .zip(scratch_pads.par_iter_mut())
            .zip(batch_eq_coeffs.par_iter())
            .map(|((helper, sp), &eq_coeff)| {
                let local = helper.poly_eval_at(
                    i_var,
                    SUMCHECK_GKR_SIMD_MPI_DEGREE,
                    &mut sp.eq_evals_at_r_simd0,
                    &mut sp.simd_var_v_evals,
                    &mut sp.simd_var_hg_evals,
                );
                let mut local_agg = [F::ChallengeField::ZERO; 4];
                for k in 0..4 {
                    local_agg[k] = eq_coeff * local[k];
                }
                local_agg
            })
            .reduce(
                || [F::ChallengeField::ZERO; 4],
                |mut a, b| {
                    for k in 0..4 {
                        a[k] += b[k];
                    }
                    a
                },
            );
        let r = transcript_io_local::<F::ChallengeField, T>(&agg, transcript);
        simd_helpers
            .par_iter_mut()
            .zip(scratch_pads.par_iter_mut())
            .for_each(|(helper, sp)| {
                helper.receive_challenge(
                    i_var,
                    r,
                    &mut sp.eq_evals_at_r_simd0,
                    &mut sp.simd_var_v_evals,
                    &mut sp.simd_var_hg_evals,
                );
            });
        all_r_simd.push(r);
    }

    // ======== Phase 3: MPI variable sumcheck (over N batch instances) ========
    let mut mpi_v_evals = Vec::with_capacity(n);
    let mut mpi_hg_evals = Vec::with_capacity(n);
    let mut mpi_eq_evals = batch_eq_coeffs;

    for j in 0..n {
        mpi_v_evals.push(scratch_pads[j].simd_var_v_evals[0]);
        mpi_hg_evals.push(
            scratch_pads[j].simd_var_hg_evals[0] * scratch_pads[j].eq_evals_at_r_simd0[0],
        );
    }

    let mut mpi_helper = SumcheckSimdProdGateHelper::<F>::new(mpi_var_num);
    let mut all_r_mpi = Vec::with_capacity(mpi_var_num);
    for i_var in 0..mpi_var_num {
        let evals = mpi_helper.poly_eval_at(
            i_var,
            SUMCHECK_GKR_SIMD_MPI_DEGREE,
            &mut mpi_eq_evals,
            &mut mpi_v_evals,
            &mut mpi_hg_evals,
        );
        let r = transcript_io_local::<F::ChallengeField, T>(&evals, transcript);
        mpi_helper.receive_challenge(
            i_var,
            r,
            &mut mpi_eq_evals,
            &mut mpi_v_evals,
            &mut mpi_hg_evals,
        );
        all_r_mpi.push(r);
    }

    let vx_claim = mpi_v_evals[0];
    transcript.append_field_element(&vx_claim);

    // ======== Phase 4: y-variable sumcheck (if needed) ========
    let mut vy_claim = None;
    if !layers[0].structure_info.skip_sumcheck_phase_two {
        // y-phase batch coefficients: eq(r_mpi_var, j) for j = 0..N-1
        let batch_eq_y = EqPolynomial::<F::ChallengeField>::build_eq_x_r(&all_r_mpi);

        scratch_pads
            .par_iter_mut()
            .zip(layers.par_iter())
            .for_each(|(sp, layer)| {
                prepare_y_vals_for_instance(
                    layer, &all_rx, &all_r_simd, &all_r_mpi, challenge, sp, vx_claim,
                );
            });

        // phase2_coef is the SAME for all instances (shared challenge values).
        let phase2_coef = scratch_pads[0].phase2_coef;

        let mut xy_helpers_y: Vec<SumcheckProductGateHelper> = (0..n)
            .map(|_| SumcheckProductGateHelper::new(input_var_num))
            .collect();

        let mut all_ry = Vec::with_capacity(input_var_num);
        for i_var in 0..input_var_num {
            let agg = xy_helpers_y
                .par_iter_mut()
                .zip(scratch_pads.par_iter())
                .zip(layers.par_iter())
                .zip(batch_eq_y.par_iter())
                .map(|(((helper, sp), layer), &eq_y)| {
                    let local_simd = helper.poly_eval_at::<F>(
                        i_var,
                        SUMCHECK_GKR_DEGREE,
                        &sp.v_evals,
                        &sp.hg_evals,
                        &layer.input_vals,
                        &sp.gate_exists,
                    );
                    let mut local_agg = [F::ChallengeField::ZERO; 3];
                    for k in 0..3 {
                        let scalar =
                            unpack_and_combine(&local_simd[k], &sp.eq_evals_at_r_simd0);
                        local_agg[k] = eq_y * scalar * phase2_coef;
                    }
                    local_agg
                })
                .reduce(
                    || [F::ChallengeField::ZERO; 3],
                    |mut a, b| {
                        for k in 0..3 {
                            a[k] += b[k];
                        }
                        a
                    },
                );
            let r = transcript_io_local::<F::ChallengeField, T>(&agg, transcript);
            xy_helpers_y
                .par_iter_mut()
                .zip(scratch_pads.par_iter_mut())
                .zip(layers.par_iter())
                .for_each(|((helper, sp), layer)| {
                    helper.receive_challenge::<F>(
                        i_var,
                        r,
                        &mut sp.v_evals,
                        &mut sp.hg_evals,
                        &layer.input_vals,
                        &mut sp.gate_exists,
                    );
                });
            all_ry.push(r);
        }

        // vy_claim = sum over instances of eq_y[j] * unpack_and_combine(v_j[0], eq_simd_j)
        let vy_sum = scratch_pads
            .par_iter()
            .zip(batch_eq_y.par_iter())
            .map(|(sp, &eq_y)| {
                eq_y * unpack_and_combine(&sp.v_evals[0], &sp.eq_evals_at_r_simd0)
            })
            .reduce(|| F::ChallengeField::ZERO, |a, b| a + b);
        vy_claim = Some(vy_sum);
        transcript.append_field_element(&vy_sum);

        // Store eq_evals_at_rx = eq(rx, .) for the next layer's alpha blending.
        // prepare_y_vals_for_instance already set eq_evals_at_rx = eq(rx, .)
        // and this value persists across the y-phase rounds (receive_ry doesn't
        // modify eq_evals_at_rx), so nothing to do here.

        *challenge = ExpanderDualVarChallenge::new(all_rx, Some(all_ry), all_r_simd, all_r_mpi);
    } else {
        *challenge = ExpanderDualVarChallenge::new(all_rx, None, all_r_simd, all_r_mpi);
    }

    (vx_claim, vy_claim)
}
