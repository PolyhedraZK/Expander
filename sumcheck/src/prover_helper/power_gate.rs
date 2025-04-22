//! This module implements helper functions for the prover side of the sumcheck protocol
//! to evaluate power gates

use std::marker::PhantomData;

use arith::{Field, SimdField};
use gkr_engine::FieldEngine;

pub(crate) struct SumcheckPowerGateHelper<const D: usize, F: FieldEngine> {
    var_num: usize,
    sumcheck_var_idx: usize,
    cur_eval_size: usize,
    field: PhantomData<F>,
}

impl<const D: usize, F: FieldEngine> SumcheckPowerGateHelper<D, F> {
    pub(crate) fn new(var_num: usize) -> Self {
        SumcheckPowerGateHelper {
            var_num,
            sumcheck_var_idx: 0,
            cur_eval_size: 1 << var_num,
            field: PhantomData,
        }
    }

    // Function to interpolate a quadratic polynomial and update an array of points
    fn interpolate_3(p_add: &[F::Field; 3], p: &mut [F::Field; D]) {
        // Calculate coefficients for the interpolating polynomial
        let p_add_coef_0 = p_add[0];
        let p_add_coef_2 = F::field_mul_circuit_field(
            &(p_add[2] - p_add[1] - p_add[1] + p_add[0]),
            &F::CircuitField::INV_2,
        );

        let p_add_coef_1 = p_add[1] - p_add_coef_0 - p_add_coef_2;

        // Update the p array by evaluating the interpolated polynomial at different points
        // and adding the results to the existing values
        p[0] += p_add_coef_0;
        p[1] += p_add_coef_0 + p_add_coef_1 + p_add_coef_2;
        p[2] += p_add_coef_0 + p_add_coef_1.double() + p_add_coef_2.double().double();
        p[3] += p_add_coef_0 + p_add_coef_1.mul_by_3() + p_add_coef_2.mul_by_3().mul_by_3();
        p[4] += p_add_coef_0
            + p_add_coef_1.double().double()
            + F::field_mul_circuit_field(&p_add_coef_2, &F::CircuitField::from(16));
        p[5] += p_add_coef_0
            + p_add_coef_1.mul_by_5()
            + F::field_mul_circuit_field(&p_add_coef_2, &F::CircuitField::from(25));
        p[6] += p_add_coef_0
            + p_add_coef_1.mul_by_3().double()
            + F::field_mul_circuit_field(&p_add_coef_2, &F::CircuitField::from(36));
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn poly_eval_at(
        &self,
        var_idx: usize,
        bk_f: &[F::Field],
        bk_hg_5: &[F::ChallengeField],
        bk_hg_1: &[F::ChallengeField],
        init_v: &[F::SimdCircuitField],
        gate_exists_5: &[bool],
        gate_exists_1: &[bool],
    ) -> [F::Field; D] {
        let mut p = [F::Field::zero(); D];
        log::trace!("bk_f: {:?}", &bk_f[..4]);
        log::trace!("bk_hg: {:?}", &bk_hg_5[..4]);
        log::trace!("init_v: {:?}", &init_v[..4]);
        if var_idx == 0 {
            let src_v = init_v;
            let eval_size = 1 << (self.var_num - var_idx - 1);
            log::trace!("Eval size: {}", eval_size);
            for i in 0..eval_size {
                if !gate_exists_5[i * 2] && !gate_exists_5[i * 2 + 1] {
                    continue;
                }
                let mut f_v = [F::SimdCircuitField::zero(); D];
                let mut hg_v = [F::ChallengeField::zero(); D];
                f_v[0] = src_v[i * 2];
                f_v[1] = src_v[i * 2 + 1];
                hg_v[0] = bk_hg_5[i * 2];
                hg_v[1] = bk_hg_5[i * 2 + 1];
                let delta_f = f_v[1] - f_v[0];
                let delta_hg = hg_v[1] - hg_v[0];

                for i in 2..D {
                    f_v[i] = f_v[i - 1] + delta_f;
                    hg_v[i] = hg_v[i - 1] + delta_hg;
                }
                for i in 0..D {
                    let pow5 = f_v[i].square().square() * f_v[i];
                    p[i] += F::simd_circuit_field_mul_challenge_field(&pow5, &hg_v[i]);
                }
            }
            let mut p_add = [F::Field::zero(); 3];
            for i in 0..eval_size {
                if !gate_exists_1[i * 2] && !gate_exists_1[i * 2 + 1] {
                    continue;
                }
                let mut f_v = [F::SimdCircuitField::zero(); 3];
                let mut hg_v = [F::ChallengeField::zero(); 3];
                f_v[0] = src_v[i * 2];
                f_v[1] = src_v[i * 2 + 1];
                hg_v[0] = bk_hg_1[i * 2];
                hg_v[1] = bk_hg_1[i * 2 + 1];
                p_add[0] += F::simd_circuit_field_mul_challenge_field(&f_v[0], &hg_v[0]);
                p_add[1] += F::simd_circuit_field_mul_challenge_field(&f_v[1], &hg_v[1]);
                let s_f_v = f_v[0] + f_v[1];
                let s_hg_v = hg_v[0] + hg_v[1];
                p_add[2] += F::simd_circuit_field_mul_challenge_field(&s_f_v, &s_hg_v);
            }

            p_add[2] = p_add[1].mul_by_6() + p_add[0].mul_by_3() - p_add[2].double();

            // interpolate p_add into 7 points
            Self::interpolate_3(&p_add, &mut p);
            p
        } else {
            let src_v = bk_f;
            let eval_size = 1 << (self.var_num - var_idx - 1);
            log::trace!("Eval size: {}", eval_size);
            for i in 0..eval_size {
                if !gate_exists_5[i * 2] && !gate_exists_5[i * 2 + 1] {
                    continue;
                }
                let mut f_v = [F::Field::zero(); D];
                let mut hg_v = [F::ChallengeField::zero(); D];
                f_v[0] = src_v[i * 2];
                f_v[1] = src_v[i * 2 + 1];
                hg_v[0] = bk_hg_5[i * 2];
                hg_v[1] = bk_hg_5[i * 2 + 1];
                let delta_f = f_v[1] - f_v[0];
                let delta_hg = hg_v[1] - hg_v[0];

                for i in 2..D {
                    f_v[i] = f_v[i - 1] + delta_f;
                    hg_v[i] = hg_v[i - 1] + delta_hg;
                }
                for i in 0..D {
                    let pow5 = f_v[i].square().square() * f_v[i];
                    p[i] += F::challenge_mul_field(&hg_v[i], &pow5);
                }
            }

            let mut p_add = [F::Field::zero(); 3];
            for i in 0..eval_size {
                if !gate_exists_1[i * 2] && !gate_exists_1[i * 2 + 1] {
                    continue;
                }
                let mut f_v = [F::Field::zero(); 3];
                let mut hg_v = [F::ChallengeField::zero(); 3];
                f_v[0] = src_v[i * 2];
                f_v[1] = src_v[i * 2 + 1];
                hg_v[0] = bk_hg_1[i * 2];
                hg_v[1] = bk_hg_1[i * 2 + 1];
                p_add[0] += F::challenge_mul_field(&hg_v[0], &f_v[0]);
                p_add[1] += F::challenge_mul_field(&hg_v[1], &f_v[1]);

                let s_f_v = f_v[0] + f_v[1];
                let s_hg_v = hg_v[0] + hg_v[1];
                p_add[2] += F::challenge_mul_field(&s_hg_v, &s_f_v);
            }
            p_add[2] = p_add[1].mul_by_6() + p_add[0].mul_by_3() - p_add[2].double();

            // interpolate p_add into 7 points
            Self::interpolate_3(&p_add, &mut p);
            p
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn receive_challenge(
        &mut self,
        var_idx: usize,
        r: F::ChallengeField,
        bk_f: &mut [F::Field],
        bk_hg_5: &mut [F::ChallengeField],
        bk_hg_1: &mut [F::ChallengeField],
        init_v: &[F::SimdCircuitField],
        gate_exists_5: &mut [bool],
        gate_exists_1: &mut [bool],
    ) {
        assert_eq!(var_idx, self.sumcheck_var_idx);
        assert!(var_idx < self.var_num);
        log::trace!("challenge eval size: {}", self.cur_eval_size);
        if var_idx == 0 {
            for i in 0..self.cur_eval_size >> 1 {
                let diff = init_v[2 * i + 1] - init_v[2 * i];
                let mul = F::simd_circuit_field_mul_challenge_field(&diff, &r);
                let init_v_0 = F::simd_circuit_field_into_field(&init_v[2 * i]);
                bk_f[i] = init_v_0 + mul;

                if !gate_exists_5[i * 2] && !gate_exists_5[i * 2 + 1] {
                    gate_exists_5[i] = false;
                    bk_hg_5[i] = F::ChallengeField::zero();
                } else {
                    gate_exists_5[i] = true;
                    bk_hg_5[i] = bk_hg_5[2 * i] + (bk_hg_5[2 * i + 1] - bk_hg_5[2 * i]) * r;
                }

                if !gate_exists_1[i * 2] && !gate_exists_1[i * 2 + 1] {
                    gate_exists_1[i] = false;
                    bk_hg_1[i] = F::ChallengeField::zero();
                } else {
                    gate_exists_1[i] = true;
                    bk_hg_1[i] = bk_hg_1[2 * i] + (bk_hg_1[2 * i + 1] - bk_hg_1[2 * i]) * r;
                }
            }
        } else {
            for i in 0..self.cur_eval_size >> 1 {
                bk_f[i] = bk_f[2 * i] + (bk_f[2 * i + 1] - bk_f[2 * i]).scale(&r);

                if !gate_exists_5[i * 2] && !gate_exists_5[i * 2 + 1] {
                    gate_exists_5[i] = false;
                    bk_hg_5[i] = F::ChallengeField::zero();
                } else {
                    gate_exists_5[i] = true;
                    bk_hg_5[i] = bk_hg_5[2 * i] + (bk_hg_5[2 * i + 1] - bk_hg_5[2 * i]) * r;
                }

                if !gate_exists_1[i * 2] && !gate_exists_1[i * 2 + 1] {
                    gate_exists_1[i] = false;
                    bk_hg_1[i] = F::ChallengeField::zero();
                } else {
                    gate_exists_1[i] = true;
                    bk_hg_1[i] = bk_hg_1[2 * i] + (bk_hg_1[2 * i + 1] - bk_hg_1[2 * i]) * r;
                }
            }
        }

        self.cur_eval_size >>= 1;
        self.sumcheck_var_idx += 1;
    }
}
