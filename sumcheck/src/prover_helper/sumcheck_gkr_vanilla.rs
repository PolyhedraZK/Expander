use arith::{Field, SimdField};
use circuit::CircuitLayer;
use config::{GKRConfig, MPIConfig};
use polynomials::EqPolynomial;

use crate::{unpack_and_combine, ProverScratchPad};

use super::{product_gate::SumcheckProductGateHelper, simd_gate::SumcheckSimdProdGateHelper};

pub(crate) struct SumcheckGkrVanillaHelper<'a, C: GKRConfig> {
    pub(crate) rx: Vec<C::ChallengeField>,
    pub(crate) ry: Vec<C::ChallengeField>,
    pub(crate) r_simd_var: Vec<C::ChallengeField>,
    pub(crate) r_mpi_var: Vec<C::ChallengeField>,

    layer: &'a CircuitLayer<C>,
    sp: &'a mut ProverScratchPad<C>,
    rz0: &'a [C::ChallengeField],
    rz1: &'a Option<Vec<C::ChallengeField>>,
    r_simd: &'a [C::ChallengeField],
    r_mpi: &'a [C::ChallengeField],
    alpha: C::ChallengeField,
    beta: Option<C::ChallengeField>,

    pub(crate) input_var_num: usize,
    pub(crate) simd_var_num: usize,

    xy_helper: SumcheckProductGateHelper,
    simd_var_helper: SumcheckSimdProdGateHelper,
    mpi_var_helper: SumcheckSimdProdGateHelper,

    mpi_config: &'a MPIConfig,
}

/// internal helper functions
impl<'a, C: GKRConfig> SumcheckGkrVanillaHelper<'a, C> {
    #[inline(always)]
    fn xy_helper_receive_challenge(&mut self, var_idx: usize, r: C::ChallengeField) {
        self.xy_helper.receive_challenge::<C>(
            var_idx,
            r,
            &mut self.sp.v_evals,
            &mut self.sp.hg_evals,
            &self.layer.input_vals,
            &mut self.sp.gate_exists_5,
        );
    }
}

/// Helper functions to be called
#[allow(clippy::too_many_arguments)]
impl<'a, C: GKRConfig> SumcheckGkrVanillaHelper<'a, C> {
    #[inline]
    pub(crate) fn new(
        layer: &'a CircuitLayer<C>,
        rz0: &'a [C::ChallengeField],
        rz1: &'a Option<Vec<C::ChallengeField>>,
        r_simd: &'a [C::ChallengeField],
        r_mpi: &'a [C::ChallengeField],
        alpha: C::ChallengeField,
        beta: Option<C::ChallengeField>,
        sp: &'a mut ProverScratchPad<C>,
        mpi_config: &'a MPIConfig,
    ) -> Self {
        let simd_var_num = C::get_field_pack_size().trailing_zeros() as usize;
        SumcheckGkrVanillaHelper {
            rx: vec![],
            ry: vec![],
            r_simd_var: vec![],
            r_mpi_var: vec![],

            layer,
            sp,
            rz0,
            rz1,
            r_simd,
            r_mpi,
            alpha,
            beta,

            input_var_num: layer.input_var_num,
            simd_var_num,

            xy_helper: SumcheckProductGateHelper::new(layer.input_var_num),
            simd_var_helper: SumcheckSimdProdGateHelper::new(simd_var_num),
            mpi_var_helper: SumcheckSimdProdGateHelper::new(
                mpi_config.world_size().trailing_zeros() as usize,
            ),
            mpi_config,
        }
    }

    pub(crate) fn poly_evals_at_rx(
        &mut self,
        var_idx: usize,
        degree: usize,
    ) -> [C::ChallengeField; 3] {
        assert!(var_idx < self.input_var_num);
        let local_vals_simd = self.xy_helper.poly_eval_at::<C>(
            var_idx,
            degree,
            &self.sp.v_evals,
            &self.sp.hg_evals,
            &self.layer.input_vals,
            &self.sp.gate_exists_5,
        );

        // SIMD
        let local_vals = local_vals_simd
            .iter()
            .map(|p| unpack_and_combine(p, &self.sp.eq_evals_at_r_simd0))
            .collect::<Vec<C::ChallengeField>>();

        // MPI
        let global_vals = self
            .mpi_config
            .coef_combine_vec(&local_vals, &self.sp.eq_evals_at_r_mpi0);
        if self.mpi_config.is_root() {
            global_vals.try_into().unwrap()
        } else {
            [C::ChallengeField::ZERO; 3]
        }
    }

    pub(crate) fn poly_evals_at_r_simd_var(
        &mut self,
        var_idx: usize,
        degree: usize,
    ) -> [C::ChallengeField; 4] {
        assert!(var_idx < self.simd_var_num);
        let local_vals = self.simd_var_helper.poly_eval_at::<C>(
            var_idx,
            degree,
            &mut self.sp.eq_evals_at_r_simd0,
            &mut self.sp.simd_var_v_evals,
            &mut self.sp.simd_var_hg_evals,
        );
        let global_vals = self
            .mpi_config
            .coef_combine_vec(&local_vals.to_vec(), &self.sp.eq_evals_at_r_mpi0);
        if self.mpi_config.is_root() {
            global_vals.try_into().unwrap()
        } else {
            [C::ChallengeField::ZERO; 4]
        }
    }

    pub(crate) fn poly_evals_at_r_mpi_var(
        &mut self,
        var_idx: usize,
        degree: usize,
    ) -> [C::ChallengeField; 4] {
        assert!(var_idx < self.mpi_config.world_size().trailing_zeros() as usize);
        self.mpi_var_helper.poly_eval_at::<C>(
            var_idx,
            degree,
            &mut self.sp.eq_evals_at_r_mpi0,
            &mut self.sp.mpi_var_v_evals,
            &mut self.sp.mpi_var_hg_evals,
        )
    }

    #[inline(always)]
    pub(crate) fn poly_evals_at_ry(
        &mut self,
        var_idx: usize,
        degree: usize,
    ) -> [C::ChallengeField; 3] {
        self.poly_evals_at_rx(var_idx, degree)
    }

    #[inline]
    pub(crate) fn receive_rx(&mut self, var_idx: usize, r: C::ChallengeField) {
        self.xy_helper_receive_challenge(var_idx, r);
        self.rx.push(r);
    }

    #[inline]
    pub(crate) fn receive_r_simd_var(&mut self, var_idx: usize, r: C::ChallengeField) {
        self.simd_var_helper.receive_challenge::<C>(
            var_idx,
            r,
            &mut self.sp.eq_evals_at_r_simd0,
            &mut self.sp.simd_var_v_evals,
            &mut self.sp.simd_var_hg_evals,
        );
        self.r_simd_var.push(r);
    }

    #[inline]
    pub(crate) fn receive_r_mpi_var(&mut self, var_idx: usize, r: C::ChallengeField) {
        self.mpi_var_helper.receive_challenge::<C>(
            var_idx,
            r,
            &mut self.sp.eq_evals_at_r_mpi0,
            &mut self.sp.mpi_var_v_evals,
            &mut self.sp.mpi_var_hg_evals,
        );
        self.r_mpi_var.push(r);
    }

    #[inline]
    pub(crate) fn receive_ry(&mut self, var_idx: usize, r: C::ChallengeField) {
        self.xy_helper_receive_challenge(var_idx, r);
        self.ry.push(r);
    }

    /// Warning:
    /// The function must be called at a specific point of the protocol, otherwise it's incorrect
    /// Consider fix this.
    pub(crate) fn vx_claim(&self) -> C::ChallengeField {
        self.sp.mpi_var_v_evals[0]
    }

    #[inline(always)]
    pub(crate) fn vy_claim(&self) -> C::ChallengeField {
        let vy_local = unpack_and_combine(&self.sp.v_evals[0], &self.sp.eq_evals_at_r_simd0);
        let summation = self
            .mpi_config
            .coef_combine_vec(&vec![vy_local], &self.sp.eq_evals_at_r_mpi0);
        if self.mpi_config.is_root() {
            summation[0]
        } else {
            C::ChallengeField::ZERO
        }
    }

    #[inline]
    pub(crate) fn prepare_simd(&mut self) {
        EqPolynomial::<C::ChallengeField>::eq_eval_at(
            self.r_simd,
            &C::ChallengeField::one(),
            &mut self.sp.eq_evals_at_r_simd0,
            &mut self.sp.eq_evals_first_half,
            &mut self.sp.eq_evals_second_half,
        );
    }

    #[inline]
    pub(crate) fn prepare_mpi(&mut self) {
        // TODO: No need to evaluate it at all world ranks, remove redundancy later.
        EqPolynomial::<C::ChallengeField>::eq_eval_at(
            self.r_mpi,
            &C::ChallengeField::one(),
            &mut self.sp.eq_evals_at_r_mpi0,
            &mut self.sp.eq_evals_first_half,
            &mut self.sp.eq_evals_second_half,
        );
    }

    #[inline]
    pub(crate) fn prepare_x_vals(&mut self) {
        let mul = &self.layer.mul;
        let add = &self.layer.add;
        let vals = &self.layer.input_vals;
        let eq_evals_at_rz0 = &mut self.sp.eq_evals_at_rz0;
        let eq_evals_at_rz1 = &mut self.sp.eq_evals_at_rz1;
        let gate_exists = &mut self.sp.gate_exists_5;
        let hg_vals = &mut self.sp.hg_evals;
        // hg_vals[0..vals.len()].fill(F::zero()); // FIXED: consider memset unsafe?
        unsafe {
            std::ptr::write_bytes(hg_vals.as_mut_ptr(), 0, vals.len());
        }
        // gate_exists[0..vals.len()].fill(false); // FIXED: consider memset unsafe?
        unsafe {
            std::ptr::write_bytes(gate_exists.as_mut_ptr(), 0, vals.len());
        }

        EqPolynomial::<C::ChallengeField>::eq_eval_at(
            self.rz0,
            &self.alpha,
            eq_evals_at_rz0,
            &mut self.sp.eq_evals_first_half,
            &mut self.sp.eq_evals_second_half,
        );

        // they should both be some or both be none though
        if self.rz1.is_some() && self.beta.is_some() {
            EqPolynomial::<C::ChallengeField>::eq_eval_at(
                self.rz1.as_ref().unwrap(),
                &self.beta.unwrap(),
                eq_evals_at_rz1,
                &mut self.sp.eq_evals_first_half,
                &mut self.sp.eq_evals_second_half,
            );

            for i in 0..1 << self.rz0.len() {
                eq_evals_at_rz0[i] += eq_evals_at_rz1[i];
            }
        }

        for g in mul.iter() {
            let r = C::challenge_mul_circuit_field(&eq_evals_at_rz0[g.o_id], &g.coef);
            hg_vals[g.i_ids[0]] += C::simd_circuit_field_mul_challenge_field(&vals[g.i_ids[1]], &r);

            gate_exists[g.i_ids[0]] = true;
        }

        for g in add.iter() {
            hg_vals[g.i_ids[0]] += C::Field::from(C::challenge_mul_circuit_field(
                &eq_evals_at_rz0[g.o_id],
                &g.coef,
            ));
            gate_exists[g.i_ids[0]] = true;
        }
    }

    #[inline]
    pub(crate) fn prepare_simd_var_vals(&mut self) {
        self.sp.simd_var_v_evals = self.sp.v_evals[0].unpack();
        self.sp.simd_var_hg_evals = self.sp.hg_evals[0].unpack();
    }

    #[inline]
    pub(crate) fn prepare_mpi_var_vals(&mut self) {
        self.mpi_config.gather_vec(
            &vec![self.sp.simd_var_v_evals[0]],
            &mut self.sp.mpi_var_v_evals,
        );
        self.mpi_config.gather_vec(
            &vec![self.sp.simd_var_hg_evals[0] * self.sp.eq_evals_at_r_simd0[0]],
            &mut self.sp.mpi_var_hg_evals,
        );
    }

    #[inline]
    pub(crate) fn prepare_y_vals(&mut self) {
        let mut v_rx_rsimd_rw = self.sp.mpi_var_v_evals[0];
        self.mpi_config.root_broadcast(&mut v_rx_rsimd_rw);

        let mul = &self.layer.mul;
        let eq_evals_at_rz0 = &self.sp.eq_evals_at_rz0;
        let eq_evals_at_rx = &mut self.sp.eq_evals_at_rx;
        let gate_exists = &mut self.sp.gate_exists_5;
        let hg_vals = &mut self.sp.hg_evals;
        let fill_len = 1 << self.rx.len();
        // hg_vals[0..fill_len].fill(F::zero()); // FIXED: consider memset unsafe?
        unsafe {
            std::ptr::write_bytes(hg_vals.as_mut_ptr(), 0, fill_len);
        }
        // gate_exists[0..fill_len].fill(false); // FIXED: consider memset unsafe?
        unsafe {
            std::ptr::write_bytes(gate_exists.as_mut_ptr(), 0, fill_len);
        }

        EqPolynomial::<C::ChallengeField>::eq_eval_at(
            &self.r_mpi_var,
            &C::ChallengeField::one(),
            &mut self.sp.eq_evals_at_r_mpi0,
            &mut self.sp.eq_evals_first_half,
            &mut self.sp.eq_evals_second_half,
        );

        // TODO: For root process, _eq_vec does not have to be recomputed
        let coef = EqPolynomial::<C::ChallengeField>::eq_vec(self.r_mpi, &self.r_mpi_var)
            * self.sp.eq_evals_at_r_simd0[0]
            * v_rx_rsimd_rw;

        EqPolynomial::<C::ChallengeField>::eq_eval_at(
            &self.rx,
            &coef,
            eq_evals_at_rx,
            &mut self.sp.eq_evals_first_half,
            &mut self.sp.eq_evals_second_half,
        );

        EqPolynomial::<C::ChallengeField>::eq_eval_at(
            &self.r_simd_var,
            &C::ChallengeField::one(),
            &mut self.sp.eq_evals_at_r_simd0,
            &mut self.sp.eq_evals_first_half,
            &mut self.sp.eq_evals_second_half,
        );

        // TODO-OPTIMIZATION: hg_vals does not have to be simd here
        for g in mul.iter() {
            hg_vals[g.i_ids[1]] += C::Field::from(C::challenge_mul_circuit_field(
                &(eq_evals_at_rz0[g.o_id] * eq_evals_at_rx[g.i_ids[0]]),
                &g.coef,
            ));
            gate_exists[g.i_ids[1]] = true;
        }
    }
}