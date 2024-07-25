use arith::{Field, SimdField};

use crate::{CircuitLayer, GkrScratchpad};

use crate::sumcheck_helper::eq_eval_at;

struct SumcheckMultiSquareHelper<const D: usize> {
    var_num: usize,
    sumcheck_var_idx: usize,
    cur_eval_size: usize,
}

impl<const D: usize> SumcheckMultiSquareHelper<D> {
    fn new(var_num: usize) -> Self {
        SumcheckMultiSquareHelper {
            var_num,
            sumcheck_var_idx: 0,
            cur_eval_size: 1 << var_num,
        }
    }

    fn poly_eval_at<F: Field>(
        &self,
        var_idx: usize,
        bk_f: &mut [F],
        bk_hg: &mut [F],
        init_v: &[F],
        gate_exists: &[bool],
    ) -> [F; D] {
        let mut p = [F::zero(); D];
        log::trace!("bk_f: {:?}", &bk_f[..4]);
        log::trace!("bk_hg: {:?}", &bk_hg[..4]);
        log::trace!("init_v: {:?}", &init_v[..4]);
        let src_v = if var_idx == 0 { init_v } else { bk_f };
        let eval_size = 1 << (self.var_num - var_idx - 1);
        log::trace!("Eval size: {}", eval_size);
        for i in 0..eval_size {
            if !gate_exists[i * 2] && !gate_exists[i * 2 + 1] {
                continue;
            }
            let mut f_v = [F::zero(); D];
            let mut hg_v = [F::zero(); D];
            f_v[0] = src_v[i * 2];
            f_v[1] = src_v[i * 2 + 1];
            hg_v[0] = bk_hg[i * 2];
            hg_v[1] = bk_hg[i * 2 + 1];
            let delta_f = f_v[1] - f_v[0];
            let delta_hg = hg_v[1] - hg_v[0];

            for i in 2..D {
                f_v[i] = f_v[i - 1] + delta_f;
                hg_v[i] = hg_v[i - 1] + delta_hg;
            }
            for i in 0..D {
                p[i] += f_v[i].square().square() * f_v[i] * hg_v[i];
            }
        }
        p
    }

    fn receive_challenge<F: Field + SimdField>(
        &mut self,
        var_idx: usize,
        r: F::Scalar,
        bk_f: &mut [F],
        bk_hg: &mut [F],
        init_v: &[F],
        gate_exists: &mut [bool],
    ) {
        assert_eq!(var_idx, self.sumcheck_var_idx);
        assert!(var_idx < self.var_num);
        log::trace!("challenge eval size: {}", self.cur_eval_size);
        for i in 0..self.cur_eval_size >> 1 {
            if !gate_exists[i * 2] && !gate_exists[i * 2 + 1] {
                gate_exists[i] = false;

                if var_idx == 0 {
                    bk_f[i] = init_v[2 * i] + (init_v[2 * i + 1] - init_v[2 * i]).scale(&r);
                } else {
                    bk_f[i] = bk_f[2 * i] + (bk_f[2 * i + 1] - bk_f[2 * i]).scale(&r);
                }

                bk_hg[i] = F::zero();
            } else {
                gate_exists[i] = true;

                if var_idx == 0 {
                    bk_f[i] = init_v[2 * i] + (init_v[2 * i + 1] - init_v[2 * i]).scale(&r);
                } else {
                    bk_f[i] = bk_f[2 * i] + (bk_f[2 * i + 1] - bk_f[2 * i]).scale(&r);
                }
                bk_hg[i] = bk_hg[2 * i] + (bk_hg[2 * i + 1] - bk_hg[2 * i]).scale(&r);
            }
        }

        self.cur_eval_size >>= 1;
        self.sumcheck_var_idx += 1;
    }
}

#[allow(dead_code)]
pub(crate) struct SumcheckGkrSquareHelper<'a, F: Field + SimdField, const D: usize> {
    pub(crate) rx: Vec<F::Scalar>,

    layer: &'a CircuitLayer<F>,
    sp: &'a mut GkrScratchpad<F>,
    rz0: &'a [F::Scalar],

    input_var_num: usize,
    output_var_num: usize,

    x_helper: SumcheckMultiSquareHelper<D>,
}

impl<'a, F: Field + SimdField, const D: usize> SumcheckGkrSquareHelper<'a, F, D> {
    pub fn new(
        layer: &'a CircuitLayer<F>,
        rz0: &'a [F::Scalar],
        sp: &'a mut GkrScratchpad<F>,
    ) -> Self {
        SumcheckGkrSquareHelper {
            rx: vec![],

            layer,
            sp,
            rz0,

            input_var_num: layer.input_var_num,
            output_var_num: layer.output_var_num,

            x_helper: SumcheckMultiSquareHelper::new(layer.input_var_num),
        }
    }

    pub fn poly_evals_at(&mut self, var_idx: usize) -> [F; D] {
        self.x_helper.poly_eval_at(
            var_idx,
            &mut self.sp.v_evals,
            &mut self.sp.hg_evals,
            &self.layer.input_vals.evals,
            &self.sp.gate_exists,
        )
    }

    pub fn receive_challenge(&mut self, var_idx: usize, r: F::Scalar) {
        self.x_helper.receive_challenge(
            var_idx,
            r,
            &mut self.sp.v_evals,
            &mut self.sp.hg_evals,
            &self.layer.input_vals.evals,
            &mut self.sp.gate_exists,
        );
        log::trace!("v_eval[0]:= {:?}", self.sp.v_evals[0]);
        self.rx.push(r);
    }

    pub fn vx_claim(&self) -> F {
        self.sp.v_evals[0]
    }

    pub fn prepare_g_x_vals(&mut self) {
        let uni = &self.layer.uni; // univariate things like square, pow5, etc.
        let add = &self.layer.add;
        let vals = &self.layer.input_vals;
        let eq_evals_at_rz0 = &mut self.sp.eq_evals_at_rz0;
        let gate_exists = &mut self.sp.gate_exists;
        let hg_vals = &mut self.sp.hg_evals;
        // hg_vals[0..vals.evals.len()].fill(F::zero()); // FIXED: consider memset unsafe?
        unsafe {
            std::ptr::write_bytes(hg_vals.as_mut_ptr(), 0, vals.evals.len());
        }
        // gate_exists[0..vals.evals.len()].fill(false); // FIXED: consider memset unsafe?
        unsafe {
            std::ptr::write_bytes(gate_exists.as_mut_ptr(), 0, vals.evals.len());
        }
        eq_eval_at(
            self.rz0,
            &F::Scalar::one(),
            eq_evals_at_rz0,
            &mut self.sp.eq_evals_first_half,
            &mut self.sp.eq_evals_second_half,
        );

        for g in uni.iter() {
            hg_vals[g.i_ids[0]] += F::from(g.coef * eq_evals_at_rz0[g.o_id]);
            gate_exists[g.i_ids[0]] = true;
        }
        for g in add.iter() {
            hg_vals[g.i_ids[0]] += F::from(g.coef * eq_evals_at_rz0[g.o_id]);
            gate_exists[g.i_ids[0]] = true;
        }
    }
}
