//! Scratch pad for prover and verifier to store intermediate values during the sumcheck protocol.

use std::cmp::max;

use arith::{ExtensionField, Field};
use circuit::Circuit;
use gkr_engine::{FieldEngine, FieldType};

#[derive(Clone, Debug, Default)]
pub struct ProverScratchPad<F: FieldEngine> {
    pub v_evals: Vec<F::Field>,
    pub hg_evals: Vec<F::Field>,
    pub simd_var_v_evals: Vec<F::ChallengeField>,
    pub simd_var_hg_evals: Vec<F::ChallengeField>,
    pub mpi_var_v_evals: Vec<F::ChallengeField>,
    pub mpi_var_hg_evals: Vec<F::ChallengeField>,

    pub eq_evals_at_rx: Vec<F::ChallengeField>,
    pub eq_evals_at_rz0: Vec<F::ChallengeField>,
    pub eq_evals_at_r_simd0: Vec<F::ChallengeField>,
    pub eq_evals_at_r_mpi0: Vec<F::ChallengeField>,
    pub eq_evals_first_half: Vec<F::ChallengeField>,
    pub eq_evals_second_half: Vec<F::ChallengeField>,

    pub gate_exists: Vec<bool>,

    pub phase2_coef: F::ChallengeField,
}

impl<F: FieldEngine> ProverScratchPad<F> {
    /// Compute the smallest power of 2 that is greater than or equal to the square root of n.
    pub fn pow2_sqrt_ceil(n: usize) -> usize {
        let mut x = 1;
        while x * x < n {
            x *= 2;
        }
        x
    }

    pub fn new(max_num_input_var: usize, max_num_output_var: usize, mpi_world_size: usize) -> Self {
        let max_input_size = 1 << max_num_input_var;
        let max_output_size = 1 << max_num_output_var;
        let max_io_size = max(max_input_size, max_output_size);
        let max_size = max(max(max_io_size, F::get_field_pack_size()), mpi_world_size);
        // Ensure scratch is at least SIMD and MPI size (not just sqrt of max_size),
        // because collectively_eval_circuit_vals_at_expander_challenge needs
        // scratch >= 1 << max(r_simd.len(), r_mpi.len()).
        let sqrt_max_size = max(
            Self::pow2_sqrt_ceil(max_size),
            max(F::get_field_pack_size(), mpi_world_size),
        );

        ProverScratchPad {
            v_evals: vec![F::Field::default(); max_input_size],
            hg_evals: vec![F::Field::default(); max_input_size],
            simd_var_v_evals: vec![F::ChallengeField::default(); F::get_field_pack_size()],
            simd_var_hg_evals: vec![F::ChallengeField::default(); F::get_field_pack_size()],
            mpi_var_v_evals: vec![F::ChallengeField::default(); mpi_world_size],
            mpi_var_hg_evals: vec![F::ChallengeField::default(); mpi_world_size],

            eq_evals_at_rx: vec![F::ChallengeField::default(); max_input_size],
            eq_evals_at_rz0: vec![F::ChallengeField::default(); max_output_size],
            eq_evals_at_r_simd0: vec![F::ChallengeField::default(); F::get_field_pack_size()],
            eq_evals_at_r_mpi0: vec![F::ChallengeField::default(); mpi_world_size],
            eq_evals_first_half: vec![F::ChallengeField::default(); sqrt_max_size],
            eq_evals_second_half: vec![F::ChallengeField::default(); sqrt_max_size],

            gate_exists: vec![false; max_input_size],
            phase2_coef: F::ChallengeField::ZERO,
        }
    }
}

/// Pre-allocated flat-buffer batch of `ProverScratchPad` instances.
///
/// Instead of N×13 heap allocations, this does exactly 3 allocations (one per
/// element type) and creates N `ProverScratchPad` instances whose `Vec` fields
/// alias into the flat buffers via `Vec::from_raw_parts`.
///
/// # Safety
/// - `ScratchPadBatch` MUST outlive all references to the scratch pads.
/// - The custom `Drop` impl forgets all inner Vecs before dropping the flat buffers.
/// - The scratch pads MUST NOT be individually dropped or moved out of `sps`.
pub struct ScratchPadBatch<F: FieldEngine> {
    /// Backing buffer for v_evals + hg_evals (F::Field elements)
    _f_buf: Vec<F::Field>,
    /// Backing buffer for all ChallengeField vecs
    _cf_buf: Vec<F::ChallengeField>,
    /// Backing buffer for gate_exists
    _b_buf: Vec<bool>,
    /// The aliased scratch pad instances
    sps: Vec<ProverScratchPad<F>>,
}

impl<F: FieldEngine> ScratchPadBatch<F> {
    /// Create a batch of `n` scratch pads sharing 3 flat buffers.
    ///
    /// Parameters match `ProverScratchPad::new`.
    pub fn new(
        n: usize,
        max_num_input_var: usize,
        max_num_output_var: usize,
        mpi_world_size: usize,
    ) -> Self {
        let max_input_size = 1 << max_num_input_var;
        let max_output_size = 1 << max_num_output_var;
        let max_io_size = max(max_input_size, max_output_size);
        let pack_size = F::get_field_pack_size();
        let max_size = max(max(max_io_size, pack_size), mpi_world_size);
        let sqrt_max_size = max(
            ProverScratchPad::<F>::pow2_sqrt_ceil(max_size),
            max(pack_size, mpi_world_size),
        );

        // Per-SP element counts
        let f_per_sp = 2 * max_input_size; // v_evals + hg_evals
        let cf_per_sp = 3 * pack_size // simd_var_v/hg + eq_at_r_simd0
            + 3 * mpi_world_size           // mpi_var_v/hg + eq_at_r_mpi0
            + max_input_size               // eq_evals_at_rx
            + max_output_size              // eq_evals_at_rz0
            + 2 * sqrt_max_size;           // eq_evals_first/second_half
        let b_per_sp = max_input_size; // gate_exists

        // Allocate flat buffers (use unsafe alloc_zeroed for large buffers to avoid sequential init)
        let mut f_buf: Vec<F::Field> = if n * f_per_sp > 65536 {
            unsafe {
                let layout = std::alloc::Layout::array::<F::Field>(n * f_per_sp).unwrap();
                let ptr = std::alloc::alloc_zeroed(layout) as *mut F::Field;
                Vec::from_raw_parts(ptr, n * f_per_sp, n * f_per_sp)
            }
        } else {
            vec![F::Field::default(); n * f_per_sp]
        };
        let mut cf_buf: Vec<F::ChallengeField> = if n * cf_per_sp > 65536 {
            unsafe {
                let layout = std::alloc::Layout::array::<F::ChallengeField>(n * cf_per_sp).unwrap();
                let ptr = std::alloc::alloc_zeroed(layout) as *mut F::ChallengeField;
                Vec::from_raw_parts(ptr, n * cf_per_sp, n * cf_per_sp)
            }
        } else {
            vec![F::ChallengeField::default(); n * cf_per_sp]
        };
        let mut b_buf: Vec<bool> = vec![false; n * b_per_sp];

        // Build aliased scratch pads
        let mut sps = Vec::with_capacity(n);
        for i in 0..n {
            let f_base = f_buf.as_mut_ptr();
            let cf_base = cf_buf.as_mut_ptr();
            let b_base = b_buf.as_mut_ptr();

            let f_off = i * f_per_sp;
            let cf_off = i * cf_per_sp;
            let b_off = i * b_per_sp;

            unsafe {
                // F::Field slices: v_evals, hg_evals
                let v_evals = Vec::from_raw_parts(
                    f_base.add(f_off),
                    max_input_size,
                    max_input_size,
                );
                let hg_evals = Vec::from_raw_parts(
                    f_base.add(f_off + max_input_size),
                    max_input_size,
                    max_input_size,
                );

                // ChallengeField slices — laid out contiguously
                let mut co = cf_off; // running offset into cf_buf

                let simd_var_v_evals = Vec::from_raw_parts(
                    cf_base.add(co), pack_size, pack_size,
                );
                co += pack_size;

                let simd_var_hg_evals = Vec::from_raw_parts(
                    cf_base.add(co), pack_size, pack_size,
                );
                co += pack_size;

                let mpi_var_v_evals = Vec::from_raw_parts(
                    cf_base.add(co), mpi_world_size, mpi_world_size,
                );
                co += mpi_world_size;

                let mpi_var_hg_evals = Vec::from_raw_parts(
                    cf_base.add(co), mpi_world_size, mpi_world_size,
                );
                co += mpi_world_size;

                let eq_evals_at_rx = Vec::from_raw_parts(
                    cf_base.add(co), max_input_size, max_input_size,
                );
                co += max_input_size;

                let eq_evals_at_rz0 = Vec::from_raw_parts(
                    cf_base.add(co), max_output_size, max_output_size,
                );
                co += max_output_size;

                let eq_evals_at_r_simd0 = Vec::from_raw_parts(
                    cf_base.add(co), pack_size, pack_size,
                );
                co += pack_size;

                let eq_evals_at_r_mpi0 = Vec::from_raw_parts(
                    cf_base.add(co), mpi_world_size, mpi_world_size,
                );
                co += mpi_world_size;

                let eq_evals_first_half = Vec::from_raw_parts(
                    cf_base.add(co), sqrt_max_size, sqrt_max_size,
                );
                co += sqrt_max_size;

                let eq_evals_second_half = Vec::from_raw_parts(
                    cf_base.add(co), sqrt_max_size, sqrt_max_size,
                );
                let _ = co + sqrt_max_size; // consumed

                // bool slice: gate_exists
                let gate_exists = Vec::from_raw_parts(
                    b_base.add(b_off), max_input_size, max_input_size,
                );

                sps.push(ProverScratchPad {
                    v_evals,
                    hg_evals,
                    simd_var_v_evals,
                    simd_var_hg_evals,
                    mpi_var_v_evals,
                    mpi_var_hg_evals,
                    eq_evals_at_rx,
                    eq_evals_at_rz0,
                    eq_evals_at_r_simd0,
                    eq_evals_at_r_mpi0,
                    eq_evals_first_half,
                    eq_evals_second_half,
                    gate_exists,
                    phase2_coef: F::ChallengeField::ZERO,
                });
            }
        }

        ScratchPadBatch {
            _f_buf: f_buf,
            _cf_buf: cf_buf,
            _b_buf: b_buf,
            sps,
        }
    }

    /// Returns a mutable slice of N scratch pads.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [ProverScratchPad<F>] {
        &mut self.sps
    }
}

impl<F: FieldEngine> Drop for ScratchPadBatch<F> {
    fn drop(&mut self) {
        // Forget all inner Vecs to prevent double-free.
        // The flat buffers (_f_buf, _cf_buf, _b_buf) own the memory
        // and will free it when this struct drops (after this fn returns).
        for sp in &mut self.sps {
            std::mem::forget(std::mem::take(&mut sp.v_evals));
            std::mem::forget(std::mem::take(&mut sp.hg_evals));
            std::mem::forget(std::mem::take(&mut sp.simd_var_v_evals));
            std::mem::forget(std::mem::take(&mut sp.simd_var_hg_evals));
            std::mem::forget(std::mem::take(&mut sp.mpi_var_v_evals));
            std::mem::forget(std::mem::take(&mut sp.mpi_var_hg_evals));
            std::mem::forget(std::mem::take(&mut sp.eq_evals_at_rx));
            std::mem::forget(std::mem::take(&mut sp.eq_evals_at_rz0));
            std::mem::forget(std::mem::take(&mut sp.eq_evals_at_r_simd0));
            std::mem::forget(std::mem::take(&mut sp.eq_evals_at_r_mpi0));
            std::mem::forget(std::mem::take(&mut sp.eq_evals_first_half));
            std::mem::forget(std::mem::take(&mut sp.eq_evals_second_half));
            std::mem::forget(std::mem::take(&mut sp.gate_exists));
        }
    }
}

#[derive(Clone, Debug)]
pub struct VerifierScratchPad<F: FieldEngine> {
    // ====== for evaluating cst, add and mul ======
    pub eq_evals_at_rz0: Vec<F::ChallengeField>,
    pub eq_evals_at_r_simd: Vec<F::ChallengeField>,
    pub eq_evals_at_r_mpi: Vec<F::ChallengeField>,

    pub eq_evals_at_rx: Vec<F::ChallengeField>,
    pub eq_evals_at_ry: Vec<F::ChallengeField>,

    pub eq_evals_first_part: Vec<F::ChallengeField>,
    pub eq_evals_second_part: Vec<F::ChallengeField>,

    pub r_simd: Vec<F::ChallengeField>,
    pub r_mpi: Vec<F::ChallengeField>,
    pub eq_r_simd_r_simd_xy: F::ChallengeField,
    pub eq_r_mpi_r_mpi_xy: F::ChallengeField,

    // ====== for deg2, deg3 eval ======
    pub gf2_deg2_eval_coef: F::ChallengeField, // 1 / x(x - 1)
    pub deg3_eval_at: [F::ChallengeField; 4],
    pub deg3_lag_denoms_inv: [F::ChallengeField; 4],
    // ====== for deg6 eval ======
    pub deg6_eval_at: [F::ChallengeField; 7],
    pub deg6_lag_denoms_inv: [F::ChallengeField; 7],
}

impl<F: FieldEngine> VerifierScratchPad<F> {
    pub fn new(circuit: &Circuit<F>, mpi_world_size: usize) -> Self {
        let mut max_num_var = circuit
            .layers
            .iter()
            .map(|layer| layer.output_var_num)
            .max()
            .unwrap();
        max_num_var = max(max_num_var, circuit.log_input_size());

        let max_io_size = 1usize << max_num_var;
        let simd_size = F::get_field_pack_size();

        let gf2_deg2_eval_coef = if F::FIELD_TYPE == FieldType::GF2Ext128 {
            (F::ChallengeField::X - F::ChallengeField::one())
                .mul_by_x()
                .inv()
                .unwrap()
        } else {
            F::ChallengeField::INV_2
        };

        let deg3_eval_at = if F::FIELD_TYPE == FieldType::GF2Ext128 {
            [
                F::ChallengeField::ZERO,
                F::ChallengeField::ONE,
                F::ChallengeField::X,
                F::ChallengeField::X.mul_by_x(),
            ]
        } else {
            [
                F::ChallengeField::ZERO,
                F::ChallengeField::ONE,
                F::ChallengeField::from(2u32),
                F::ChallengeField::from(3u32),
            ]
        };

        let mut deg3_lag_denoms_inv = [F::ChallengeField::ZERO; 4];
        for i in 0..4 {
            let mut denominator = F::ChallengeField::ONE;
            for j in 0..4 {
                if j == i {
                    continue;
                }
                denominator *= deg3_eval_at[i] - deg3_eval_at[j];
            }
            deg3_lag_denoms_inv[i] = denominator.inv().unwrap();
        }

        let deg6_eval_at = [
            F::ChallengeField::ZERO,
            F::ChallengeField::ONE,
            F::ChallengeField::from(2u32),
            F::ChallengeField::from(3u32),
            F::ChallengeField::from(4u32),
            F::ChallengeField::from(5u32),
            F::ChallengeField::from(6u32),
        ];

        let mut deg6_lag_denoms_inv = [F::ChallengeField::ZERO; 7];
        for i in 0..7 {
            let mut denominator = F::ChallengeField::ONE;
            for j in 0..7 {
                if j == i {
                    continue;
                }
                denominator *= deg6_eval_at[i] - deg6_eval_at[j];
            }
            deg6_lag_denoms_inv[i] = denominator.inv().unwrap();
        }

        Self {
            eq_evals_at_rz0: vec![F::ChallengeField::zero(); max_io_size],
            eq_evals_at_r_simd: vec![F::ChallengeField::zero(); simd_size],
            eq_evals_at_r_mpi: vec![F::ChallengeField::zero(); mpi_world_size],

            eq_evals_at_rx: vec![F::ChallengeField::zero(); max_io_size],
            eq_evals_at_ry: vec![F::ChallengeField::zero(); max_io_size],

            eq_evals_first_part: vec![
                F::ChallengeField::zero();
                max(max(max_io_size, simd_size), mpi_world_size)
            ],
            eq_evals_second_part: vec![
                F::ChallengeField::zero();
                max(max(max_io_size, simd_size), mpi_world_size)
            ],

            r_simd: vec![],
            r_mpi: vec![],
            eq_r_simd_r_simd_xy: F::ChallengeField::zero(),
            eq_r_mpi_r_mpi_xy: F::ChallengeField::zero(),

            gf2_deg2_eval_coef,
            deg3_eval_at,
            deg3_lag_denoms_inv,
            deg6_eval_at,
            deg6_lag_denoms_inv,
        }
    }
}
