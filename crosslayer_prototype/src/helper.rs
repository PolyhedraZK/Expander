use crate::{
    CrossLayerCircuitEvals, CrossLayerConnections, CrossLayerProverScratchPad, GenericLayer,
};

use arith::{ExtensionField, Field, SimdField};
use gkr_field_config::{FieldType, GKRFieldConfig};
use polynomials::EqPolynomial;
use sumcheck::unpack_and_combine;

pub(crate) struct MultilinearProductHelper {}

impl MultilinearProductHelper {
    // Sumcheck the product of two multi-linear polynomials f and h_g
    //
    // Inputs:
    // - var_idx: the index of the variable to evaluate
    // - degree: the degree of the result univariate polynomial
    // - bk_f: bookkeeping table of f(x)
    // - bk_hg: bookkeeping table of h_g(x)
    // - init_v: input values; will be processed iff var_idex == 0
    // Output:
    // - the univariate polynomial that prover sends to the verifier
    #[inline]
    pub(crate) fn poly_eval_at<C: GKRFieldConfig>(
        var_num: usize,
        var_idx: usize,
        degree: usize,
        bk_f: &[C::Field],
        bk_hg: &[C::Field],
        init_v: &[C::SimdCircuitField],
    ) -> [C::Field; 3] {
        assert_eq!(degree, 2);

        let mut p0 = C::Field::zero();
        let mut p1 = C::Field::zero();
        let mut p2 = C::Field::zero();

        let eval_size = 1 << (var_num - var_idx - 1);
        if var_idx == 0 {
            // this is the first layer, we are able to accelerate by
            // avoiding the extension field operations
            for i in 0..eval_size {
                let f_v_0 = init_v[i * 2];
                let f_v_1 = init_v[i * 2 + 1];
                let hg_v_0 = bk_hg[i * 2];
                let hg_v_1 = bk_hg[i * 2 + 1];
                p0 += C::field_mul_simd_circuit_field(&hg_v_0, &f_v_0);
                p1 += C::field_mul_simd_circuit_field(&hg_v_1, &f_v_1);
                p2 += C::field_mul_simd_circuit_field(&(hg_v_0 + hg_v_1), &(f_v_0 + f_v_1));
            }
        } else {
            for i in 0..eval_size {
                let f_v_0 = bk_f[i * 2];
                let f_v_1 = bk_f[i * 2 + 1];
                let hg_v_0 = bk_hg[i * 2];
                let hg_v_1 = bk_hg[i * 2 + 1];
                p0 += f_v_0 * hg_v_0;
                p1 += f_v_1 * hg_v_1;
                p2 += (f_v_0 + f_v_1) * (hg_v_0 + hg_v_1);
            }
        }

        if C::FIELD_TYPE == FieldType::GF2 {
            // over GF2_128, the three points are at 0, 1 and X
            let p2x = p2.mul_by_x();
            let p2x2 = p2x.mul_by_x();
            let linear_term = p1 + p0 + p2;
            p2 = p2x2 + linear_term.mul_by_x() + p0;
        } else {
            // when Field size > 2, the three points are 0, 1, -2
            p2 = p1.mul_by_6() + p0.mul_by_3() - p2.double();
        }

        [p0, p1, p2]
    }

    // process the challenge and update the bookkeeping tables for f and h_g accordingly
    #[inline]
    pub(crate) fn receive_challenge<C: GKRFieldConfig>(
        var_num: usize,
        var_idx: usize,
        r: C::ChallengeField,
        bk_f: &mut [C::Field],
        bk_hg: &mut [C::Field],
        init_v: &[C::SimdCircuitField],
    ) {
        assert!(var_idx < var_num);

        let eval_size = 1 << (var_num - var_idx - 1);
        if var_idx == 0 {
            for i in 0..eval_size {
                bk_f[i] = C::field_add_simd_circuit_field(
                    &C::simd_circuit_field_mul_challenge_field(
                        &(init_v[i * 2 + 1] - init_v[i * 2]),
                        &r,
                    ),
                    &init_v[i * 2],
                );
                bk_hg[i] = bk_hg[2 * i] + (bk_hg[2 * i + 1] - bk_hg[2 * i]).scale(&r);
            }
        } else {
            for i in 0..eval_size {
                bk_f[i] = bk_f[2 * i] + (bk_f[2 * i + 1] - bk_f[2 * i]).scale(&r);
                bk_hg[i] = bk_hg[2 * i] + (bk_hg[2 * i + 1] - bk_hg[2 * i]).scale(&r);
            }
        }
    }
}

pub(crate) struct SumcheckSimdProdGateHelper {}

// The logic is exactly the same as SumcheckProductGateHelper, but field types are different
impl SumcheckSimdProdGateHelper {
    #[inline]
    pub(crate) fn poly_eval_at<C: GKRFieldConfig>(
        var_num: usize,
        var_idx: usize,
        degree: usize,
        bk_eq: &mut [C::ChallengeField],
        bk_f: &mut [C::ChallengeField],
        bk_hg: &mut [C::ChallengeField],
    ) -> [C::ChallengeField; 4] {
        assert_eq!(degree, 3);
        let mut p0 = C::ChallengeField::zero();
        let mut p1 = C::ChallengeField::zero();
        let mut p2 = C::ChallengeField::zero();
        let mut p3 = C::ChallengeField::zero();

        let eval_size = 1 << (var_num - var_idx - 1);

        if C::FIELD_TYPE == FieldType::GF2 {
            for i in 0..eval_size {
                let eq_v_0 = bk_eq[i * 2];
                let eq_v_1 = bk_eq[i * 2 + 1];
                let f_v_0 = bk_f[i * 2];
                let f_v_1 = bk_f[i * 2 + 1];
                let hg_v_0 = bk_hg[i * 2];
                let hg_v_1 = bk_hg[i * 2 + 1];

                p0 += eq_v_0 * f_v_0 * hg_v_0;
                p1 += eq_v_1 * f_v_1 * hg_v_1;

                let eq_linear = (eq_v_1 - eq_v_0).mul_by_x();
                let f_linear = (f_v_1 - f_v_0).mul_by_x();
                let hg_linear = (hg_v_1 - hg_v_0).mul_by_x();

                // evaluated at x and x^2 for p2 and p3
                p2 += (eq_linear + eq_v_0) * (f_linear + f_v_0) * (hg_linear + hg_v_0);
                p3 += (eq_linear.mul_by_x() + eq_v_0)
                    * (f_linear.mul_by_x() + f_v_0)
                    * (hg_linear.mul_by_x() + hg_v_0);
            }
        } else {
            for i in 0..eval_size {
                let eq_v_0 = bk_eq[i * 2];
                let eq_v_1 = bk_eq[i * 2 + 1];
                let f_v_0 = bk_f[i * 2];
                let f_v_1 = bk_f[i * 2 + 1];
                let hg_v_0 = bk_hg[i * 2];
                let hg_v_1 = bk_hg[i * 2 + 1];

                p0 += eq_v_0 * f_v_0 * hg_v_0;
                p1 += eq_v_1 * f_v_1 * hg_v_1;

                // evaluated at 2 and 3 for p2 and p3
                let tmp0 = eq_v_1 - eq_v_0;
                let tmp1 = f_v_1 - f_v_0;
                let tmp2 = hg_v_1 - hg_v_0;
                let tmp3 = eq_v_1 + tmp0;
                let tmp4 = f_v_1 + tmp1;
                let tmp5 = hg_v_1 + tmp2;

                p2 += tmp3 * tmp4 * tmp5;
                p3 += (tmp3 + tmp0) * (tmp4 + tmp1) * (tmp5 + tmp2);
            }
        }

        [p0, p1, p2, p3]
    }

    #[inline]
    pub(crate) fn receive_challenge<C: GKRFieldConfig>(
        var_num: usize,
        var_idx: usize,
        r: C::ChallengeField,
        bk_eq: &mut [C::ChallengeField],
        bk_f: &mut [C::ChallengeField],
        bk_hg: &mut [C::ChallengeField],
    ) {
        assert!(var_idx < var_num);

        let eval_size = 1 << (var_num - var_idx - 1);
        for i in 0..eval_size {
            bk_eq[i] = bk_eq[2 * i] + (bk_eq[2 * i + 1] - bk_eq[2 * i]) * r;
            bk_f[i] = bk_f[2 * i] + (bk_f[2 * i + 1] - bk_f[2 * i]) * r;
            bk_hg[i] = bk_hg[2 * i] + (bk_hg[2 * i + 1] - bk_hg[2 * i]) * r;
        }
    }
}

pub(crate) struct CrossLayerScatterHelper<'a, C: GKRFieldConfig> {
    pub(crate) input_layer_var_num: usize,
    pub(crate) rx: Vec<C::ChallengeField>,
    pub(crate) ry: Vec<C::ChallengeField>,
    pub(crate) r_simd_next: Vec<C::ChallengeField>,
    pub(crate) r_relays_next: Vec<(usize, Vec<C::ChallengeField>)>,

    layer: &'a GenericLayer<C>,
    rz0: &'a [C::ChallengeField],
    r_simd: &'a [C::ChallengeField],
    connections: &'a CrossLayerConnections,
    circuit_vals: &'a CrossLayerCircuitEvals<C>,
    sp: &'a mut CrossLayerProverScratchPad<C>,
}

/// Helper functions to be called
#[allow(clippy::too_many_arguments)]
impl<'a, C: GKRFieldConfig> CrossLayerScatterHelper<'a, C> {
    #[inline]
    pub(crate) fn new(
        layer: &'a GenericLayer<C>,
        rz0: &'a [C::ChallengeField],
        r_simd: &'a [C::ChallengeField],
        connections: &'a CrossLayerConnections,
        circuit_vals: &'a CrossLayerCircuitEvals<C>,
        sp: &'a mut CrossLayerProverScratchPad<C>,
    ) -> Self {
        CrossLayerScatterHelper {
            input_layer_var_num: layer.input_layer_size.trailing_zeros() as usize,
            rx: vec![],
            ry: vec![],
            r_simd_next: vec![],
            r_relays_next: vec![],

            layer,
            rz0,
            r_simd,
            connections,
            circuit_vals,
            sp,
        }
    }

    pub(crate) fn poly_evals_at_rx(
        &mut self,
        var_idx: usize,
        degree: usize,
    ) -> [C::ChallengeField; 3] {
        assert!(degree == 2);
        // layer_id - 1
        let mut p3 = MultilinearProductHelper::poly_eval_at::<C>(
            self.input_layer_var_num,
            var_idx,
            degree,
            &self.sp.v_evals,
            &self.sp.hg_evals,
            &self.circuit_vals.vals[self.layer.layer_id - 1],
        );

        // [0, layer_id - 2]
        for i_layer in 0..(self.layer.layer_id - 1) {
            let cross_layer_size = self.sp.cross_layer_sizes[i_layer];
            if cross_layer_size > 0 {
                if var_idx < cross_layer_size.trailing_zeros() as usize {
                    let p3_at_layer_i = MultilinearProductHelper::poly_eval_at::<C>(
                        cross_layer_size.trailing_zeros() as usize,
                        var_idx,
                        degree,
                        &self.sp.cross_layer_evals[i_layer],
                        &self.sp.cross_layer_hg_evals[i_layer],
                        &self.sp.cross_layer_circuit_vals[i_layer],
                    );
                    for i in 0..3 {
                        p3[i] += p3_at_layer_i[i];
                    }
                } else {
                    for p in p3.iter_mut() {
                        *p += self.sp.cross_layer_completed_values[i_layer];
                    }
                }
            }
        }

        p3.iter()
            .map(|p| unpack_and_combine(p, &self.sp.eq_evals_at_r_simd))
            .collect::<Vec<C::ChallengeField>>()
            .try_into()
            .unwrap()
    }

    pub(crate) fn poly_evals_at_r_simd_var(
        &mut self,
        var_idx: usize,
        degree: usize,
    ) -> [C::ChallengeField; 4] {
        SumcheckSimdProdGateHelper::poly_eval_at::<C>(
            C::get_field_pack_size().trailing_zeros() as usize,
            var_idx,
            degree,
            &mut self.sp.eq_evals_at_r_simd,
            &mut self.sp.simd_var_v_evals,
            &mut self.sp.simd_var_hg_evals,
        )
    }

    #[inline(always)]
    pub(crate) fn poly_evals_at_ry(
        &mut self,
        var_idx: usize,
        degree: usize,
    ) -> [C::ChallengeField; 3] {
        assert!(degree == 2);
        let p3 = MultilinearProductHelper::poly_eval_at::<C>(
            self.input_layer_var_num,
            var_idx,
            degree,
            &self.sp.v_evals,
            &self.sp.hg_evals,
            &self.circuit_vals.vals[self.layer.layer_id - 1],
        );
        p3.iter()
            .map(|p| {
                unpack_and_combine(
                    &(C::challenge_mul_field(&self.sp.phase2_coef, p)),
                    &self.sp.eq_evals_at_r_simd,
                )
            })
            .collect::<Vec<C::ChallengeField>>()
            .try_into()
            .unwrap()
    }

    // Returns which relay layer has ended, and the final claim, can be empty
    #[inline]
    pub(crate) fn receive_rx(&mut self, var_idx: usize, r: C::ChallengeField) {
        MultilinearProductHelper::receive_challenge::<C>(
            self.input_layer_var_num,
            var_idx,
            r,
            &mut self.sp.v_evals,
            &mut self.sp.hg_evals,
            &self.circuit_vals.vals[self.layer.layer_id - 1],
        );

        for i_layer in 0..(self.layer.layer_id - 1) {
            let cross_layer_size = self.sp.cross_layer_sizes[i_layer];
            if cross_layer_size > 0 {
                if var_idx < cross_layer_size.trailing_zeros() as usize {
                    MultilinearProductHelper::receive_challenge::<C>(
                        cross_layer_size.trailing_zeros() as usize,
                        var_idx,
                        r,
                        &mut self.sp.cross_layer_evals[i_layer],
                        &mut self.sp.cross_layer_hg_evals[i_layer],
                        &self.circuit_vals.vals[i_layer],
                    );

                    if var_idx == cross_layer_size.trailing_zeros() as usize - 1 {
                        // save the completed value
                        self.r_relays_next.push((i_layer, self.rx.clone()));
                        self.sp.cross_layer_completed_values[i_layer] = self.sp.cross_layer_evals
                            [i_layer][0]
                            * self.sp.cross_layer_hg_evals[i_layer][0];
                    }
                } else {
                    // for extra bits in sumcheck, we require it to be 1
                    self.sp.cross_layer_completed_values[i_layer] =
                        C::challenge_mul_field(&r, &self.sp.cross_layer_completed_values[i_layer]);
                }
            }
        }

        self.rx.push(r);
    }

    #[inline]
    pub(crate) fn receive_r_simd_var(&mut self, var_idx: usize, r: C::ChallengeField) {
        SumcheckSimdProdGateHelper::receive_challenge::<C>(
            C::get_field_pack_size().trailing_zeros() as usize,
            var_idx,
            r,
            &mut self.sp.eq_evals_at_r_simd,
            &mut self.sp.simd_var_v_evals,
            &mut self.sp.simd_var_hg_evals,
        );
        self.r_simd_next.push(r);
    }

    #[inline]
    pub(crate) fn receive_ry(&mut self, var_idx: usize, r: C::ChallengeField) {
        MultilinearProductHelper::receive_challenge::<C>(
            self.input_layer_var_num,
            var_idx,
            r,
            &mut self.sp.v_evals,
            &mut self.sp.hg_evals,
            &self.circuit_vals.vals[self.layer.layer_id - 1],
        );
        self.ry.push(r);
    }

    pub(crate) fn vx_claims(&self) -> Vec<(usize, C::ChallengeField)> {
        // TODO-Optimization: Maybe it's better to reduce simd for each relay layer individually and
        // return the result
        let mut claims = vec![(
            self.layer.layer_id - 1,
            unpack_and_combine(&self.sp.v_evals[0], &self.r_simd_next),
        )];
        for (i_layer, cross_layer_size) in self.sp.cross_layer_sizes.iter().enumerate() {
            if *cross_layer_size > 0 {
                claims.push((
                    i_layer,
                    unpack_and_combine(&self.sp.cross_layer_evals[i_layer][0], &self.r_simd_next),
                ));
            }
        }
        claims
    }

    #[inline(always)]
    pub(crate) fn vy_claim(&self) -> C::ChallengeField {
        unpack_and_combine(
            &self.sp.v_evals[0],
            self.sp.eq_evals_at_r_simd_at_layer[self.layer.layer_id].as_slice(),
        )
    }

    #[inline]
    pub(crate) fn prepare_simd(&mut self) {
        if self.layer.layer_id == self.sp.eq_evals_at_r_simd_at_layer.len() - 1 {
            EqPolynomial::<C::ChallengeField>::eq_eval_at(
                self.r_simd,
                &C::ChallengeField::one(),
                &mut self.sp.eq_evals_at_r_simd,
                &mut self.sp.eq_evals_first_half,
                &mut self.sp.eq_evals_second_half,
            );
        } else {
            // TODO: No need to actually clone
            self.sp.eq_evals_at_r_simd =
                self.sp.eq_evals_at_r_simd_at_layer[self.layer.layer_id].clone();
        }
    }

    #[inline]
    pub(crate) fn prepare_x_vals(&mut self) {
        let eq_evals_at_rz = &mut self.sp.eq_evals_at_rz0;
        EqPolynomial::eq_eval_at(
            self.rz0,
            &C::ChallengeField::ONE,
            eq_evals_at_rz,
            &mut self.sp.eq_evals_first_half,
            &mut self.sp.eq_evals_second_half,
        );

        // processing the relay layers
        let layers_connected_to = &self.connections.connections[self.layer.layer_id];

        #[allow(clippy::needless_range_loop)]
        for i_layer in 0..(self.layer.layer_id - 1) {
            let connections_at_i_layer = &layers_connected_to[i_layer];
            let cross_layer_size = &mut self.sp.cross_layer_sizes[i_layer];
            let cir_vals = &mut self.sp.cross_layer_circuit_vals[i_layer];
            let vals = &mut self.sp.cross_layer_evals[i_layer];
            let hg_vals = &mut self.sp.cross_layer_hg_evals[i_layer];
            *cross_layer_size = 0;
            cir_vals.clear();
            vals.clear();
            hg_vals.clear();

            if !connections_at_i_layer.is_empty() {
                *cross_layer_size = connections_at_i_layer.len().next_power_of_two();
                // TODO: Allocate this in scratchpad
                cir_vals.resize(*cross_layer_size, C::SimdCircuitField::ZERO);
                vals.resize(*cross_layer_size, C::Field::ZERO);
                hg_vals.resize(*cross_layer_size, C::Field::ZERO);

                for (idx, (o_id, i_id)) in connections_at_i_layer.iter().enumerate() {
                    cir_vals[idx] = self.circuit_vals.vals[i_layer][*i_id];
                    // Do nothing to vals[idx] here, it will be processed later in folding
                    hg_vals[idx] = C::Field::from(eq_evals_at_rz[*o_id]);
                }
            }
        }

        // processing the input layer
        let mul = &self.layer.mul_gates;
        let add = &self.layer.add_gates;
        let vals = &self.circuit_vals.vals[self.layer.layer_id - 1];
        let hg_vals = &mut self.sp.hg_evals;
        unsafe {
            std::ptr::write_bytes(hg_vals.as_mut_ptr(), 0, vals.len());
        }

        for g in mul.iter() {
            let r = C::challenge_mul_circuit_field(&eq_evals_at_rz[g.o_id], &g.coef);
            hg_vals[g.i_ids[0]] += C::simd_circuit_field_mul_challenge_field(&vals[g.i_ids[1]], &r);
        }

        for g in add.iter() {
            hg_vals[g.i_ids[0]] += C::Field::from(C::challenge_mul_circuit_field(
                &eq_evals_at_rz[g.o_id],
                &g.coef,
            ));
        }
    }

    #[inline]
    pub(crate) fn prepare_simd_var_vals(&mut self) {
        self.sp.simd_var_v_evals = self.sp.v_evals[0].unpack();
        self.sp.simd_var_hg_evals = self.sp.hg_evals[0].unpack();

        for (i_layer, cross_layer_size) in self.sp.cross_layer_sizes.iter().enumerate() {
            if *cross_layer_size > 0 {
                let simd_var_v_evals = self.sp.cross_layer_evals[i_layer][0].unpack();
                let simd_var_hg_evals = self.sp.cross_layer_hg_evals[i_layer][0].unpack();

                for i in 0..simd_var_v_evals.len() {
                    self.sp.simd_var_v_evals[i] += simd_var_v_evals[i];
                    self.sp.simd_var_hg_evals[i] += simd_var_hg_evals[i];
                }
            }
        }
    }

    #[inline]
    pub(crate) fn prepare_y_vals(&mut self) {
        self.sp.phase2_coef = self.sp.simd_var_v_evals[0] * self.sp.eq_evals_at_r_simd[0];

        let mul = &self.layer.mul_gates;
        let eq_evals_at_rz = &self.sp.eq_evals_at_rz0;
        let eq_evals_at_rx = &mut self.sp.eq_evals_at_rx;
        let hg_vals = &mut self.sp.hg_evals;
        let fill_len = 1 << self.rx.len();
        // hg_vals[0..fill_len].fill(F::zero()); // FIXED: consider memset unsafe?
        unsafe {
            std::ptr::write_bytes(hg_vals.as_mut_ptr(), 0, fill_len);
        }

        EqPolynomial::<C::ChallengeField>::eq_eval_at(
            &self.rx,
            &C::ChallengeField::ONE,
            eq_evals_at_rx,
            &mut self.sp.eq_evals_first_half,
            &mut self.sp.eq_evals_second_half,
        );

        EqPolynomial::<C::ChallengeField>::eq_eval_at(
            &self.r_simd_next,
            &C::ChallengeField::ONE,
            &mut self.sp.eq_evals_at_r_simd_at_layer[self.layer.layer_id],
            &mut self.sp.eq_evals_first_half,
            &mut self.sp.eq_evals_second_half,
        );

        for g in mul.iter() {
            hg_vals[g.i_ids[1]] += C::Field::from(C::challenge_mul_circuit_field(
                &(eq_evals_at_rz[g.o_id] * eq_evals_at_rx[g.i_ids[0]]),
                &g.coef,
            ));
        }
    }
}

pub(crate) struct CrossLayerGatherHelper<'a, C: GKRFieldConfig> {
    pub(crate) rx: Vec<C::ChallengeField>,

    layer: &'a GenericLayer<C>,
    rz0: &'a [C::ChallengeField],
    rz1: &'a [C::ChallengeField],
    r_relays: &'a [(usize, Vec<C::ChallengeField>)],
    alpha: &'a C::ChallengeField, // alpha is the random value multiplied to V(rz1)
    betas: &'a [C::ChallengeField], /* betas random value multiplied to the claims from the
                                   * previous non-zero relay layer */
    connections: &'a CrossLayerConnections,
    circuit_vals: &'a CrossLayerCircuitEvals<C>,

    sp: &'a mut CrossLayerProverScratchPad<C>,

    pub(crate) cur_layer_var_num: usize,
}

#[allow(clippy::too_many_arguments)]
impl<'a, C: GKRFieldConfig> CrossLayerGatherHelper<'a, C> {
    pub fn new(
        layer: &'a GenericLayer<C>,
        rz0: &'a [C::ChallengeField],
        rz1: &'a [C::ChallengeField],
        r_relays: &'a [(usize, Vec<C::ChallengeField>)],
        alpha: &'a C::ChallengeField,
        betas: &'a [C::ChallengeField],
        connections: &'a CrossLayerConnections,
        circuit_vals: &'a CrossLayerCircuitEvals<C>,
        sp: &'a mut CrossLayerProverScratchPad<C>,
    ) -> Self {
        CrossLayerGatherHelper {
            rx: vec![],
            layer,
            rz0,
            rz1,
            r_relays,
            alpha,
            betas,
            connections,
            circuit_vals,
            sp,
            cur_layer_var_num: layer.layer_size.trailing_zeros() as usize,
        }
    }

    pub(crate) fn poly_evals_at_rx(
        &mut self,
        var_idx: usize,
        degree: usize,
    ) -> [C::ChallengeField; 3] {
        MultilinearProductHelper::poly_eval_at::<C>(
            self.cur_layer_var_num,
            var_idx,
            degree,
            &self.sp.v_evals,
            &self.sp.hg_evals,
            &self.circuit_vals.vals[self.layer.layer_id],
        )
        .iter()
        .map(|p| p.unpack()[0])
        .collect::<Vec<C::ChallengeField>>()
        .try_into()
        .unwrap()
    }

    pub(crate) fn receive_rx(&mut self, var_idx: usize, r: C::ChallengeField) {
        MultilinearProductHelper::receive_challenge::<C>(
            self.cur_layer_var_num,
            var_idx,
            r,
            &mut self.sp.v_evals,
            &mut self.sp.hg_evals,
            &self.circuit_vals.vals[self.layer.layer_id],
        );
        self.rx.push(r);
    }

    pub(crate) fn vx_claim(&self) -> C::ChallengeField {
        self.sp.v_evals[0].unpack()[0]
    }

    #[inline]
    pub(crate) fn prepare_x_vals(&mut self) {
        let hg_vals = &mut self.sp.hg_evals;
        let eq_evals_at_rz = &mut self.sp.eq_evals_at_rz0;

        EqPolynomial::eq_eval_at(
            self.rz0,
            &C::ChallengeField::ONE,
            eq_evals_at_rz,
            &mut self.sp.eq_evals_first_half,
            &mut self.sp.eq_evals_second_half,
        );
        for i in 0..self.rz0.len() {
            hg_vals[i] += C::Field::from(eq_evals_at_rz[i]);
        }

        // second claim from the previous layer
        EqPolynomial::eq_eval_at(
            self.rz1,
            self.alpha,
            eq_evals_at_rz,
            &mut self.sp.eq_evals_first_half,
            &mut self.sp.eq_evals_second_half,
        );
        for i in 0..self.rz0.len() {
            hg_vals[i] += C::Field::from(eq_evals_at_rz[i]);
        }

        for (layer_idx, (out_layer_id, claim)) in self.r_relays.iter().enumerate() {
            EqPolynomial::eq_eval_at(
                claim,
                &self.betas[layer_idx],
                eq_evals_at_rz,
                &mut self.sp.eq_evals_first_half,
                &mut self.sp.eq_evals_second_half,
            );

            for (gate_idx, (_o_id, i_id)) in self.connections.connections[*out_layer_id]
                [self.layer.layer_id]
                .iter()
                .enumerate()
            {
                hg_vals[*i_id] += C::Field::from(eq_evals_at_rz[gate_idx]);
            }
        }
    }
}
