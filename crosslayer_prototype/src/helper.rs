use std::vec;

use crate::{circuit, CrossLayerCircuitEvals, CrossLayerConnections, CrossLayerProverScratchPad, GenericLayer};

use arith::Field;
use gkr_field_config::{FieldType, GKRFieldConfig};

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
        assert_ne!(C::FIELD_TYPE, FieldType::GF2);
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
    pub(crate) rx: Vec<C::ChallengeField>,
    pub(crate) ry: Vec<C::ChallengeField>,
    pub(crate) r_relays_next: Vec<(usize, Vec<C::ChallengeField>)>,

    layer: &'a GenericLayer<C>,
    rz0: &'a [C::ChallengeField],
    r_relays: &'a [(usize, Vec<C::ChallengeField>)],
    connections: &'a CrossLayerConnections,
    circuit_vals: &'a CrossLayerCircuitEvals<C>,
    sp: &'a mut CrossLayerProverScratchPad<C>,

    pub(crate) input_layer_var_num: usize,
}

/// Helper functions to be called
#[allow(clippy::too_many_arguments)]
impl<'a, C: GKRFieldConfig> CrossLayerScatterHelper<'a, C> {
    #[inline]
    pub(crate) fn new(
        layer: &'a GenericLayer<C>,
        rz0: &'a [C::ChallengeField],
        r_relays: &'a [(usize, Vec<C::ChallengeField>)],
        connections: &'a CrossLayerConnections,
        circuit_vals: &'a CrossLayerCircuitEvals<C>,
        sp: &'a mut CrossLayerProverScratchPad<C>,
    ) -> Self {
        CrossLayerScatterHelper {
            rx: vec![],
            ry: vec![],
            r_relays_next: vec![],

            layer,
            rz0,
            r_relays,
            connections,
            circuit_vals,
            sp,

            input_layer_var_num: layer.input_layer_size.trailing_zeros() as usize,
        }
    }

    pub(crate) fn poly_evals_at_rx(
        &mut self,
        var_idx: usize,
        degree: usize,
    ) -> [C::ChallengeField; 3] {
        todo!()
    }

    #[inline(always)]
    pub(crate) fn poly_evals_at_ry(
        &mut self,
        var_idx: usize,
        degree: usize,
    ) -> [C::ChallengeField; 3] {
        todo!()
    }

    // Returns which relay layer has ended, and the final claim, can be empty
    #[inline]
    pub(crate) fn receive_rx(&mut self, var_idx: usize, r: C::ChallengeField) -> Vec<(usize, C::ChallengeField)> {
        todo!()
    }

    #[inline]
    pub(crate) fn receive_ry(&mut self, var_idx: usize, r: C::ChallengeField) {
        todo!()
    }

    pub(crate) fn vx_claim(&self) -> C::ChallengeField {
        todo!()
    }

    #[inline(always)]
    pub(crate) fn vy_claim(&self) -> C::ChallengeField {
        todo!()
    }

    #[inline]
    pub(crate) fn prepare_x_vals(&mut self) {
        todo!()
    }

    #[inline]
    pub(crate) fn prepare_y_vals(&mut self) {
        todo!()
    }
}

pub(crate) struct CrossLayerGatherHelper<'a, C: GKRFieldConfig> {
    pub(crate) rx: Vec<C::ChallengeField>,
    
    layer: &'a GenericLayer<C>,
    rz0: &'a [C::ChallengeField],
    rz1: &'a [C::ChallengeField],
    r_relays: &'a [(usize, Vec<C::ChallengeField>)],
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
        todo!()
    }

    pub(crate) fn receive_rx(&mut self, var_idx: usize, r: C::ChallengeField) {
        todo!()
    }

    pub(crate) fn vx_claim(&self) -> C::ChallengeField {
        todo!()
    } 

    #[inline]
    pub(crate) fn prepare_x_vals(&mut self) {
        todo!()
    }
}