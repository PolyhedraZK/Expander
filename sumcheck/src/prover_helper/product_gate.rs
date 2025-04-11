//! This module implements helper functions for the prover side of the sumcheck protocol
//! to evaluate Mul gates

use std::ops::{Add, Mul};

use arith::{ExtensionField, Field, SimdField};
use env_logger::init;

pub(crate) struct SumcheckProductGateHelper {
    var_num: usize,
}

impl SumcheckProductGateHelper {
    pub(crate) fn new(var_num: usize) -> Self {
        SumcheckProductGateHelper { var_num }
    }

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
    pub(crate) fn poly_eval_at<VField: Field, EvalField: Field>(
        &self,
        var_idx: usize,
        degree: usize,
        bk_f: &[EvalField],
        bk_hg: &[EvalField],
        init_v: &[VField],
        gate_exists: &[bool],
    ) -> [EvalField; 3] 
    where
        EvalField: Mul<VField, Output = EvalField>,
    {
        assert_eq!(degree, 2);

        let mut p0 = EvalField::zero();
        let mut p1 = EvalField::zero();
        let mut p2 = EvalField::zero();
        log::trace!("bk_f: {:?}", &bk_f[..4]);
        log::trace!("bk_hg: {:?}", &bk_hg[..4]);
        log::trace!("init_v: {:?}", &init_v[..4]);

        let eval_size = 1 << (self.var_num - var_idx - 1);
        log::trace!("Eval size: {}", eval_size);

        // TODO: merge situation
        if var_idx == 0 {
            // this is the first layer, we are able to accelerate by
            // avoiding the extension field operations
            for i in 0..eval_size {
                if !gate_exists[i * 2] && !gate_exists[i * 2 + 1] {
                    continue;
                }

                let f_v_0 = init_v[i * 2];
                let f_v_1 = init_v[i * 2 + 1];
                let hg_v_0 = bk_hg[i * 2];
                let hg_v_1 = bk_hg[i * 2 + 1];

                p0 += hg_v_0 * f_v_0;
                log::trace!(
                    "p0.v += {:?} * {:?} = {:?}",
                    f_v_0,
                    hg_v_0,
                    hg_v_0 * f_v_0 + p1
                );
                p1 += hg_v_1 * f_v_1;
                p2 += (hg_v_0 + hg_v_1) * (f_v_0 + f_v_1);
            }
        } else {
            // for the rest of layers we use extension field operations.
            for i in 0..eval_size {
                if !gate_exists[i * 2] && !gate_exists[i * 2 + 1] {
                    continue;
                }

                let f_v_0 = bk_f[i * 2];
                let f_v_1 = bk_f[i * 2 + 1];
                let hg_v_0 = bk_hg[i * 2];
                let hg_v_1 = bk_hg[i * 2 + 1];
                p0 += f_v_0 * hg_v_0;
                log::trace!(
                    "p0.v+= {:?} * {:?} =  {:?}",
                    f_v_0,
                    hg_v_0,
                    f_v_0 * hg_v_0 + p1
                );
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

    #[inline]
    pub(crate) fn poly_eval_at_<C: GKRFieldConfig, FF: Field>(
        eval_size: usize,
        bk_f: &[FF],
        bk_hg: &[C::Field],
        gate_exists: &[bool],
    ) -> [C::Field; 3] 
    where
        C::Field: Mul<FF, Output = C::Field>,
    {
        let mut p0 = C::Field::zero();
        let mut p1 = C::Field::zero();
        let mut p2 = C::Field::zero();

        log::trace!("bk_f: {:?}", &bk_f[..4]);
        log::trace!("bk_hg: {:?}", &bk_hg[..4]);
        
        for i in 0..eval_size {
            if !gate_exists[i * 2] && !gate_exists[i * 2 + 1] {
                continue;
            }

            let f_v_0 = bk_f[i * 2];
            let f_v_1 = bk_f[i * 2 + 1];
            let hg_v_0 = bk_hg[i * 2];
            let hg_v_1 = bk_hg[i * 2 + 1];
            p0 += hg_v_0 * f_v_0;
            log::trace!(
                "p0.v+= {:?} * {:?} =  {:?}",
                f_v_0,
                hg_v_0,
                hg_v_0 * f_v_0 + p1
            );
            p1 += hg_v_1 * f_v_1;
            p2 += (hg_v_0 + hg_v_1) * (f_v_0 + f_v_1);
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

    #[inline]
    pub(crate) fn receive_challenge_f<C: GKRFieldConfig>(
        eval_size: usize,
        r: C::ChallengeField,
        bk_f: &mut [C::Field],
    ) {
        for i in 0..eval_size {
            bk_f[i] = bk_f[2 * i] + (bk_f[2 * i + 1] - bk_f[2 * i]).scale(&r);
        }
    }

    #[inline]
    pub(crate) fn receive_challenge_hg<C: GKRFieldConfig>(
        eval_size: usize,
        r: C::ChallengeField,
        bk_hg: &mut [C::Field],
        gate_exists: &mut [bool],
    ) {
        for i in 0..eval_size {
            if !gate_exists[i * 2] && !gate_exists[i * 2 + 1] {
                gate_exists[i] = false;
                bk_hg[i] = C::Field::zero();
            }
            else {
                gate_exists[i] = true;
                bk_hg[i] = bk_hg[2 * i] + (bk_hg[2 * i + 1] - bk_hg[2 * i]).scale(&r);
            }
        }
    }

    #[inline]
    pub(crate) fn receive_challenge<C: GKRFieldConfig>(
        &mut self,
        var_idx: usize,
        r: C::ChallengeField,
        bk_f: &mut [C::Field],
        bk_hg: &mut [C::Field],
        init_v: &[C::SimdCircuitField],
        gate_exists: &mut [bool],
    ) {
        assert!(var_idx < self.var_num);
        let eval_size = 1 << (self.var_num - var_idx - 1);

        if var_idx == 0 {
            Self::receive_challenge_f_init::<C, C::SimdCircuitField>(
                eval_size,
                r,
                bk_f,
                init_v,
            );
        } else {
            Self::receive_challenge_f::<C>(
                eval_size,
                r,
                bk_f,
            );
        }

        Self::receive_challenge_hg::<C>(
            eval_size,
            r,
            bk_hg,
            gate_exists,
        );
    }

    #[inline]
    pub(crate) fn receive_challenge_f_init<C: GKRFieldConfig, VF: Field>(
        eval_size: usize,
        r: C::ChallengeField,
        bk_f: &mut [C::Field],
        init_v: &[VF],
    ) 
    where
        C::ChallengeField: 
            Mul<VF, Output = C::Field>,
            // PackMul<VF, Output = C::Field>,
        C::Field: Add<VF, Output = C::Field>,
    {
        // TODO: used for self parallel but not used currently
        // let base_size = C::base_size();
        // if base_size > 1 {
        //     for i in (0..eval_size).step_by(base_size) {
        //         let base_i = i / base_size * 2;
        //         let bk = &mut bk_f[i..i + base_size];
        //         r.pack_mul(init_v[base_i + 1] - init_v[base_i], bk);
        //         bk.pack_add_assign(init_v[base_i]);
        //     }
        // }
        // else {
            for (i, bk) in bk_f.iter_mut().enumerate().take(eval_size) {
                *bk = r * (init_v[2 * i + 1] - init_v[2 * i]) + init_v[2 * i];
                // *bk = C::challenge_mul_field(&r, &(init_v[2 * i + 1] - init_v[2 * i])) + init_v[2 * i];
            }
        // }
    }
}
