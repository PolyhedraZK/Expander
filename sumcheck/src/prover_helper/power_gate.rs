//! This module implements helper functions for the prover side of the sumcheck protocol
//! to evaluate power gates

use arith::{Field, SimdField};

pub(crate) struct SumcheckPowerGateHelper<const D: usize> {
    var_num: usize,
    sumcheck_var_idx: usize,
    cur_eval_size: usize,
}

impl<const D: usize> SumcheckPowerGateHelper<D> {
    pub(crate) fn new(var_num: usize) -> Self {
        SumcheckPowerGateHelper {
            var_num,
            sumcheck_var_idx: 0,
            cur_eval_size: 1 << var_num,
        }
    }

    // Function to interpolate a quadratic polynomial and update an array of points
    fn interpolate_3<EvalField: ExtensionField>(p_add: &[EvalField; 3], p: &mut [EvalField; D]) 
    where
        EvalField: Mul<BaseField, Output = EvalField>,
    {
        // Calculate coefficients for the interpolating polynomial
        let p_add_coef_0 = p_add[0];
        let p_add_coef_2 = (p_add[2] - p_add[1] - p_add[1] + p_add[0]) * EvalField::BaseField::INV_2;

        let p_add_coef_1 = p_add[1] - p_add_coef_0 - p_add_coef_2;

        // Update the p array by evaluating the interpolated polynomial at different points
        // and adding the results to the existing values
        p[0] += p_add_coef_0;
        p[1] += p_add_coef_0 + p_add_coef_1 + p_add_coef_2;
        p[2] += p_add_coef_0 + p_add_coef_1.double() + p_add_coef_2.double().double();
        p[3] += p_add_coef_0 + p_add_coef_1.mul_by_3() + p_add_coef_2.mul_by_3().mul_by_3();
        p[4] += p_add_coef_0
            + p_add_coef_1.double().double()
            + p_add_coef_2 * EvalField::BaseField::from(16);
        p[5] += p_add_coef_0
            + p_add_coef_1.mul_by_5()
            + p_add_coef_2 * EvalField::BaseField::from(25);
        p[6] += p_add_coef_0
            + p_add_coef_1.mul_by_3().double()
            + p_add_coef_2 * EvalField::BaseField::from(36);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn poly_eval_at<EvalField: Field, ChallengeField: Field, VField: Field>(
        &self,
        var_idx: usize,
        bk_f: &[EvalField],
        bk_hg_5: &[ChallengeField],
        bk_hg_1: &[ChallengeField],
        init_v: &[VField],
        gate_exists_5: &[bool],
        gate_exists_1: &[bool],
    ) -> [EvalField; D] 
    where
        EvalField:
            Mul<ChallengeField, Output = EvalField> +
            Mul<VField, Output = EvalField>,
    {
        let mut p = [EvalField::zero(); D];
        log::trace!("bk_f: {:?}", &bk_f[..4]);
        log::trace!("bk_hg: {:?}", &bk_hg_5[..4]);
        log::trace!("init_v: {:?}", &init_v[..4]);

        let src_v;
        let ZERO;
        if var_idx == 0 {
            src_v = init_v;
            ZERO = VField::ZERO;
        }
        else {
            src_v = bk_f;
            ZERO = EvalField::ZERO;
        }

        let eval_size = 1 << (self.var_num - var_idx - 1);
        log::trace!("Eval size: {}", eval_size);
        for i in 0..eval_size {
            if !gate_exists_5[i * 2] && !gate_exists_5[i * 2 + 1] {
                continue;
            }
            let mut f_v = [ZERO; D];
            let mut hg_v = [ChallengeField::ZERO; D];
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
                p[i] += hg_v[i] * pow5;
            }
        }
        let mut p_add = [EvalField::ZERO; 3];
        for i in 0..eval_size {
            if !gate_exists_1[i * 2] && !gate_exists_1[i * 2 + 1] {
                continue;
            }
            let mut f_v = [ZERO; 3];
            let mut hg_v = [C::ChallengeField::ZERO; 3];
            f_v[0] = src_v[i * 2];
            f_v[1] = src_v[i * 2 + 1];
            hg_v[0] = bk_hg_1[i * 2];
            hg_v[1] = bk_hg_1[i * 2 + 1];
            p_add[0] += hg_v[0] * f_v[0];
            p_add[1] += hg_v[1] * f_v[1];
            p_add[2] += (hg_v[0] + hg_v[1]) * (f_v[0] + f_v[1]);
        }

        p_add[2] = p_add[1].mul_by_6() + p_add[0].mul_by_3() - p_add[2].double();

        // interpolate p_add into 7 points
        Self::interpolate_3(&p_add, &mut p);
        p
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn receive_challenge<EvalField: Field, ChallengeField: Field, VField: Field>(
        &mut self,
        var_idx: usize,
        r: ChallengeField,
        bk_f: &mut [EvalField],
        bk_hg_5: &mut [ChallengeField],
        bk_hg_1: &mut [ChallengeField],
        init_v: &[VField],
        gate_exists_5: &mut [bool],
        gate_exists_1: &mut [bool],
    ) 
    where
        EvalField:
            Mul<ChallengeField, Output = EvalField> +
            Add<VField, Output = EvalField> +
            Mul<VField, Output = EvalField>,
    {
        assert_eq!(var_idx, self.sumcheck_var_idx);
        assert!(var_idx < self.var_num);
        log::trace!("challenge eval size: {}", self.cur_eval_size);
        self.cur_eval_size >>= 1;
        if var_idx == 0 {
            for i in 0..self.cur_eval_size {
                bk_f[i] = (init_v[2 * i + 1] - init_v[2 * i]) * r + init_v[2 * i];
            }
        else {
            for i in 0..self.cur_eval_size >> 1 {
                bk_f[i] = bk_f[2 * i] + (bk_f[2 * i + 1] - bk_f[2 * i]).scale(&r);
        }
        for i in 0..self.cur_eval_size >> 1 {
            if !gate_exists_5[i * 2] && !gate_exists_5[i * 2 + 1] {
                gate_exists_5[i] = false;
                bk_hg_5[i] = ChallengeField::ZERO;
            } else {
                gate_exists_5[i] = true;
                bk_hg_5[i] = bk_hg_5[2 * i] + (bk_hg_5[2 * i + 1] - bk_hg_5[2 * i]) * r;
            }

            if !gate_exists_1[i * 2] && !gate_exists_1[i * 2 + 1] {
                gate_exists_1[i] = false;
                bk_hg_1[i] = ChallengeField::ZERO;
            } else {
                gate_exists_1[i] = true;
                bk_hg_1[i] = bk_hg_1[2 * i] + (bk_hg_1[2 * i + 1] - bk_hg_1[2 * i]) * r;
            }
        }

        self.sumcheck_var_idx += 1;
    }
}
