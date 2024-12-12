use crate::{CrossLayerCircuitEvals, CrossLayerConnections, CrossLayerProverScratchPad, GenericLayer};

use arith::Field;
use gkr_field_config::{FieldType, GKRFieldConfig};
use polynomials::EqPolynomial;

pub(crate) struct MultilinearProductHelper {
}

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
        bk_f: &[C::ChallengeField],
        bk_hg: &[C::ChallengeField],
    ) -> [C::ChallengeField; 3] {
        assert_eq!(degree, 2);

        let mut p0 = C::ChallengeField::zero();
        let mut p1 = C::ChallengeField::zero();
        let mut p2 = C::ChallengeField::zero();

        let eval_size = 1 << (var_num - var_idx - 1);
        for i in 0..eval_size {

            let f_v_0 = bk_f[i * 2];
            let f_v_1 = bk_f[i * 2 + 1];
            let hg_v_0 = bk_hg[i * 2];
            let hg_v_1 = bk_hg[i * 2 + 1];
            p0 += f_v_0 * hg_v_0;
            p1 += f_v_1 * hg_v_1;
            p2 += (f_v_0 + f_v_1) * (hg_v_0 + hg_v_1);
        }
        // assert_ne!(C::FIELD_TYPE, FieldType::GF2);
        p2 = p1.mul_by_6() + p0.mul_by_3() - p2.double();
        [p0, p1, p2]
    }

    // process the challenge and update the bookkeeping tables for f and h_g accordingly
    #[inline]
    pub(crate) fn receive_challenge<C: GKRFieldConfig>(
        var_num: usize,
        var_idx: usize,
        r: C::ChallengeField,
        bk_f: &mut [C::ChallengeField],
        bk_hg: &mut [C::ChallengeField],
    ) {
        assert!(var_idx < var_num);

        let eval_size = 1 << (var_num - var_idx - 1);
        for i in 0..eval_size {
            bk_f[i] = bk_f[2 * i] + (bk_f[2 * i + 1] - bk_f[2 * i]) * r;
            bk_hg[i] = bk_hg[2 * i] + (bk_hg[2 * i + 1] - bk_hg[2 * i]) * r;
        }
    }
}

pub(crate) struct CrossLayerScatterHelper<'a, C: GKRFieldConfig> {
    pub(crate) input_layer_var_num: usize,
    pub(crate) rx: Vec<C::ChallengeField>,
    pub(crate) ry: Vec<C::ChallengeField>,
    pub(crate) r_relays_next: Vec<(usize, Vec<C::ChallengeField>)>,

    layer: &'a GenericLayer<C>,
    rz0: &'a [C::ChallengeField],
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
        connections: &'a CrossLayerConnections,
        circuit_vals: &'a CrossLayerCircuitEvals<C>,
        sp: &'a mut CrossLayerProverScratchPad<C>,
    ) -> Self {
        CrossLayerScatterHelper {
            input_layer_var_num: layer.input_layer_size.trailing_zeros() as usize,
            rx: vec![],
            ry: vec![],
            r_relays_next: vec![],

            layer,
            rz0,
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
        );

        // [0, layer_id - 2]
        for i_layer in 0..(self.layer.layer_id - 1) {
            let cross_layer_size = self.sp.cross_layer_sizes[i_layer];
            if cross_layer_size > 0 && var_idx < cross_layer_size.trailing_zeros() as usize {
                let p3_at_layer_i = MultilinearProductHelper::poly_eval_at::<C>(
                    cross_layer_size.trailing_zeros() as usize,
                    var_idx, 
                    degree, 
                    &self.sp.cross_layer_evals[i_layer], 
                    &self.sp.cross_layer_hg_evals[i_layer],
                );
                for i in 0..3 {
                    p3[i] += p3_at_layer_i[i]; // TODO: Need a random coefficient here.
                }
            }
        }

        p3
    }

    #[inline(always)]
    pub(crate) fn poly_evals_at_ry(
        &mut self,
        var_idx: usize,
        degree: usize,
    ) -> [C::ChallengeField; 3] {
        assert!(degree == 2);
        MultilinearProductHelper::poly_eval_at::<C>(
            self.input_layer_var_num, 
            var_idx, 
            degree, 
            &self.sp.v_evals, 
            &self.sp.hg_evals,
        )
    }

    // Returns which relay layer has ended, and the final claim, can be empty
    #[inline]
    pub(crate) fn receive_rx(&mut self, var_idx: usize, r: C::ChallengeField) -> Vec<(usize, C::ChallengeField)> {
        MultilinearProductHelper::receive_challenge::<C>(
            self.input_layer_var_num,
            var_idx,
            r,
            &mut self.sp.v_evals,
            &mut self.sp.hg_evals,
        );

        let mut ret = vec![];
        for i_layer in 0..(self.layer.layer_id - 1) {
            let cross_layer_size = self.sp.cross_layer_sizes[i_layer];
            if cross_layer_size > 0 && var_idx < cross_layer_size.trailing_zeros() as usize {
                MultilinearProductHelper::receive_challenge::<C>(
                    cross_layer_size.trailing_zeros() as usize,
                    var_idx,
                    r,
                    &mut self.sp.cross_layer_evals[i_layer],
                    &mut self.sp.cross_layer_hg_evals[i_layer],
                );
            }

            if var_idx == cross_layer_size.trailing_zeros() as usize {
                ret.push((i_layer, self.sp.cross_layer_evals[i_layer][0]));
                self.r_relays_next.push((i_layer, self.rx.clone()));
            }
        }

        self.rx.push(r);
        ret
    }

    #[inline]
    pub(crate) fn receive_ry(&mut self, var_idx: usize, r: C::ChallengeField) {
        MultilinearProductHelper::receive_challenge::<C>(
            self.input_layer_var_num,
            var_idx,
            r,
            &mut self.sp.v_evals,
            &mut self.sp.hg_evals,
        );
        self.ry.push(r);
    }

    pub(crate) fn vx_claim(&self) -> C::ChallengeField {
        self.sp.v_evals[0]
    }

    #[inline(always)]
    pub(crate) fn vy_claim(&self) -> C::ChallengeField {
        self.sp.v_evals[0] // same as vx_claim, must be called at a different time
    }

    #[inline]
    pub(crate) fn prepare_x_vals(&mut self) {
        let eq_evals_at_rz = &mut self.sp.eq_evals_at_rz0;
        EqPolynomial::eq_eval_at(self.rz0, &C::ChallengeField::ONE, eq_evals_at_rz, &mut self.sp.eq_evals_first_half, &mut self.sp.eq_evals_second_half);

        // processing the relay layers
        let layers_connected_to = &self.connections.connections[self.layer.layer_id];
        for i_layer in 0..(self.layer.layer_id - 1) {
            let connections_at_i_layer = &layers_connected_to[i_layer];
            let cross_layer_size = &mut self.sp.cross_layer_sizes[i_layer];
            let vals = &mut self.sp.cross_layer_evals[i_layer];
            let hg_vals = &mut self.sp.cross_layer_hg_evals[i_layer];
            *cross_layer_size = 0;
            vals.clear();
            hg_vals.clear();
            
            if !connections_at_i_layer.is_empty() {
                *cross_layer_size = connections_at_i_layer.len().next_power_of_two();
                vals.resize(*cross_layer_size, C::ChallengeField::ZERO);
                hg_vals.resize(*cross_layer_size, C::ChallengeField::ZERO);

                for (idx, (o_id, i_id)) in connections_at_i_layer.into_iter().enumerate() {
                    vals[idx] = self.circuit_vals.vals[i_layer][*i_id];
                    hg_vals[idx] = eq_evals_at_rz[*o_id];
                }    
            }
        }

        // processing the input layer
        let mul = &self.layer.mul_gates;
        let add = &self.layer.add_gates;
        let vals = &self.circuit_vals.vals[self.layer.layer_id - 1];
        let hg_vals: &mut Vec<<C as GKRFieldConfig>::ChallengeField> = &mut self.sp.hg_evals;
        unsafe {
            std::ptr::write_bytes(hg_vals.as_mut_ptr(), 0, vals.len());
        }

        for g in mul.iter() {
            let r = eq_evals_at_rz[g.o_id] * g.coef;
            hg_vals[g.i_ids[0]] += vals[g.i_ids[1]] * r;
        }

        for g in add.iter() {
            hg_vals[g.i_ids[0]] += eq_evals_at_rz[g.o_id] * g.coef;
        }

        self.sp.v_evals = vals.clone(); // TODO: Remove unnecessary clone
    }

    #[inline]
    pub(crate) fn prepare_y_vals(&mut self) {
        self.sp.phase2_coef = self.sp.v_evals[0];
        
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

        for g in mul.iter() {
            hg_vals[g.i_ids[1]] += eq_evals_at_rz[g.o_id] * eq_evals_at_rx[g.i_ids[0]] * &g.coef;
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
    betas: &'a [C::ChallengeField], // betas random value multiplied to the claims from the previous non-zero relay layer
    connections: &'a CrossLayerConnections,
    circuit_vals: &'a CrossLayerCircuitEvals<C>,

    sp: &'a mut CrossLayerProverScratchPad<C>,

    pub(crate) cur_layer_var_num: usize,
}

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
        )
    }

    pub(crate) fn receive_rx(&mut self, var_idx: usize, r: C::ChallengeField) {
        MultilinearProductHelper::receive_challenge::<C>(
            self.cur_layer_var_num,
            var_idx,
            r,
            &mut self.sp.v_evals,
            &mut self.sp.hg_evals,
        );
        self.rx.push(r);
    }

    pub(crate) fn vx_claim(&self) -> C::ChallengeField {
        self.sp.v_evals[0]
    } 

    #[inline]
    pub(crate) fn prepare_x_vals(&mut self) {
        let hg_vals = &mut self.sp.hg_evals;
        let eq_evals_at_rz = &mut self.sp.eq_evals_at_rz0;

        EqPolynomial::eq_eval_at(
            self.rz0, 
            &C::ChallengeField::ONE, 
            hg_vals, 
            &mut self.sp.eq_evals_first_half,
            &mut self.sp.eq_evals_second_half,
        );

        // second claim from the previous layer
        EqPolynomial::eq_eval_at(
            self.rz1, 
            &self.alpha, 
            eq_evals_at_rz, 
            &mut self.sp.eq_evals_first_half,
            &mut self.sp.eq_evals_second_half,
        );
        for i in 0..self.rz0.len() {
            hg_vals[i] += eq_evals_at_rz[i];
        }

        
        for (layer_idx, (out_layer_id, claim)) in self.r_relays.iter().enumerate() {
            EqPolynomial::eq_eval_at(
                claim, 
                &self.betas[layer_idx], 
                eq_evals_at_rz, 
                &mut self.sp.eq_evals_first_half,
                &mut self.sp.eq_evals_second_half,
            );

            for (gate_idx, (_o_id, i_id)) in self.connections.connections[*out_layer_id][self.layer.layer_id].iter().enumerate() {
                hg_vals[*i_id] += eq_evals_at_rz[gate_idx];
            }
        }

        self.sp.v_evals = self.circuit_vals.vals[self.layer.layer_id].clone(); // TODO: Remove unnecessary clone
    }
}