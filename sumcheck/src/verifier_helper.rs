use arith::{ExtensionField, Field};
use circuit::{CircuitLayer, CoefType, GateAdd, GateConst, GateMul, GateUni};
use gkr_field_config::{FieldType, GKRFieldConfig};
use polynomials::EqPolynomial;

use crate::{scratch_pad::VerifierScratchPad, unpack_and_combine};

#[derive(Default)]
pub struct GKRVerifierHelper {}

impl GKRVerifierHelper {
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn prepare_layer<C: GKRFieldConfig>(
        layer: &CircuitLayer<C>,
        alpha: &Option<C::ChallengeField>,
        rz0: &[C::ChallengeField],
        rz1: &Option<Vec<C::ChallengeField>>,
        r_simd: &Vec<C::ChallengeField>,
        r_mpi: &Vec<C::ChallengeField>,
        sp: &mut VerifierScratchPad<C>,
        is_output_layer: bool,
    ) {
        assert_eq!(alpha.is_none(), rz1.is_none());

        if is_output_layer {
            EqPolynomial::<C::ChallengeField>::eq_eval_at(
                rz0,
                &C::ChallengeField::ONE,
                &mut sp.eq_evals_at_rz0,
                &mut sp.eq_evals_first_part,
                &mut sp.eq_evals_second_part,
            );
        } else {
            // use results from previous layer
            let output_len = 1 << rz0.len();
            sp.eq_evals_at_rz0[..output_len].copy_from_slice(&sp.eq_evals_at_rx[..output_len]);
            if alpha.is_some() && rz1.is_some() {
                let alpha = alpha.unwrap();
                for i in 0..(1usize << layer.output_var_num) {
                    sp.eq_evals_at_rz0[i] += alpha * sp.eq_evals_at_ry[i];
                }
            }
        }

        EqPolynomial::<C::ChallengeField>::eq_eval_at(
            r_simd,
            &C::ChallengeField::ONE,
            &mut sp.eq_evals_at_r_simd,
            &mut sp.eq_evals_first_part,
            &mut sp.eq_evals_second_part,
        );

        EqPolynomial::<C::ChallengeField>::eq_eval_at(
            r_mpi,
            &C::ChallengeField::ONE,
            &mut sp.eq_evals_at_r_mpi,
            &mut sp.eq_evals_first_part,
            &mut sp.eq_evals_second_part,
        );

        sp.r_simd = r_simd;
        sp.r_mpi = r_mpi;
    }

    #[inline(always)]
    pub fn eval_cst<C: GKRFieldConfig>(
        cst_gates: &[GateConst<C>],
        public_input: &[C::SimdCircuitField],
        sp: &VerifierScratchPad<C>,
    ) -> C::ChallengeField {
        let mut v = C::ChallengeField::zero();

        let mpi_world_size = sp.eq_evals_at_r_mpi.len();
        let local_input_size = public_input.len() / mpi_world_size;

        for cst_gate in cst_gates {
            let tmp = match cst_gate.coef_type {
                CoefType::PublicInput(input_idx) => {
                    let mut input = vec![];
                    for i in 0..mpi_world_size {
                        input.push(public_input[i * local_input_size + input_idx]);
                    }

                    // mpi combined
                    let input_mpi_combined: C::Field = input
                        .iter()
                        .zip(&sp.eq_evals_at_r_mpi)
                        .map(|(v, c)| C::simd_circuit_field_mul_challenge_field(v, c))
                        .sum();

                    // simd combined
                    sp.eq_evals_at_rz0[cst_gate.o_id]
                        * unpack_and_combine::<C::Field>(
                            &input_mpi_combined,
                            &sp.eq_evals_at_r_simd,
                        )
                }
                _ => C::challenge_mul_circuit_field(
                    &sp.eq_evals_at_rz0[cst_gate.o_id],
                    &cst_gate.coef,
                ),
            };
            v += tmp;
        }

        v
    }

    #[inline(always)]
    pub fn eval_add<C: GKRFieldConfig>(
        add_gates: &[GateAdd<C>],
        sp: &VerifierScratchPad<C>,
    ) -> C::ChallengeField {
        let mut v = C::ChallengeField::zero();
        for add_gate in add_gates {
            v += sp.eq_evals_at_rz0[add_gate.o_id]
                * C::challenge_mul_circuit_field(
                    &sp.eq_evals_at_rx[add_gate.i_ids[0]],
                    &add_gate.coef,
                );
        }
        v * sp.eq_r_simd_r_simd_xy * sp.eq_r_mpi_r_mpi_xy
    }

    #[inline(always)]
    pub fn eval_mul<C: GKRFieldConfig>(
        mul_gates: &[GateMul<C>],
        sp: &VerifierScratchPad<C>,
    ) -> C::ChallengeField {
        let mut v = C::ChallengeField::zero();
        for mul_gate in mul_gates {
            let tmp = sp.eq_evals_at_rx[mul_gate.i_ids[0]]
                * C::challenge_mul_circuit_field(
                    &sp.eq_evals_at_ry[mul_gate.i_ids[1]],
                    &mul_gate.coef,
                );
            v += sp.eq_evals_at_rz0[mul_gate.o_id] * tmp;
        }
        v * sp.eq_r_simd_r_simd_xy * sp.eq_r_mpi_r_mpi_xy
    }

    /// GKR2 equivalent of `eval_add`. (Note that GKR2 uses pow1 gates instead of add gates)
    #[inline(always)]
    pub fn eval_pow_1<C: GKRFieldConfig>(
        gates: &[GateUni<C>],
        sp: &VerifierScratchPad<C>,
    ) -> C::ChallengeField {
        let mut v = C::ChallengeField::zero();
        for gate in gates {
            // Gates of type 12346 represent an add gate
            if gate.gate_type == 12346 {
                v += sp.eq_evals_at_rz0[gate.o_id]
                    * C::challenge_mul_circuit_field(&sp.eq_evals_at_rx[gate.i_ids[0]], &gate.coef);
            }
        }
        v * sp.eq_r_simd_r_simd_xy * sp.eq_r_mpi_r_mpi_xy
    }

    #[inline(always)]
    pub fn eval_pow_5<C: GKRFieldConfig>(
        gates: &[GateUni<C>],
        sp: &VerifierScratchPad<C>,
    ) -> C::ChallengeField {
        let mut v = C::ChallengeField::zero();
        for gate in gates {
            // Gates of type 12345 represent a pow5 gate
            if gate.gate_type == 12345 {
                v += sp.eq_evals_at_rz0[gate.o_id]
                    * C::challenge_mul_circuit_field(&sp.eq_evals_at_rx[gate.i_ids[0]], &gate.coef);
            }
        }
        v * sp.eq_r_simd_r_simd_xy * sp.eq_r_mpi_r_mpi_xy
    }

    #[inline(always)]
    pub fn set_rx<C: GKRFieldConfig>(rx: &[C::ChallengeField], sp: &mut VerifierScratchPad<C>) {
        EqPolynomial::<C::ChallengeField>::eq_eval_at(
            rx,
            &C::ChallengeField::ONE,
            &mut sp.eq_evals_at_rx,
            &mut sp.eq_evals_first_part,
            &mut sp.eq_evals_second_part,
        );
    }

    #[inline(always)]
    pub fn set_r_simd_xy<C: GKRFieldConfig>(
        r_simd_xy: &[C::ChallengeField],
        sp: &mut VerifierScratchPad<C>,
    ) {
        sp.eq_r_simd_r_simd_xy = EqPolynomial::<C::ChallengeField>::eq_vec(
            unsafe { sp.r_simd.as_ref().unwrap() },
            r_simd_xy,
        );
    }

    #[inline(always)]
    pub fn set_r_mpi_xy<C: GKRFieldConfig>(
        r_mpi_xy: &[C::ChallengeField],
        sp: &mut VerifierScratchPad<C>,
    ) {
        sp.eq_r_mpi_r_mpi_xy = EqPolynomial::<C::ChallengeField>::eq_vec(
            unsafe { sp.r_mpi.as_ref().unwrap() },
            r_mpi_xy,
        );
    }

    #[inline(always)]
    pub fn set_ry<C: GKRFieldConfig>(ry: &[C::ChallengeField], sp: &mut VerifierScratchPad<C>) {
        EqPolynomial::<C::ChallengeField>::eq_eval_at(
            ry,
            &C::ChallengeField::ONE,
            &mut sp.eq_evals_at_ry,
            &mut sp.eq_evals_first_part,
            &mut sp.eq_evals_second_part,
        );
    }

    #[inline(always)]
    pub fn degree_2_eval<C: GKRFieldConfig>(
        ps: &[C::ChallengeField],
        x: C::ChallengeField,
        sp: &VerifierScratchPad<C>,
    ) -> C::ChallengeField {
        assert_eq!(ps.len(), 3);
        let p0 = ps[0];
        let p1 = ps[1];
        let p2 = ps[2];

        if C::FIELD_TYPE == FieldType::GF2 {
            let tmp = p0 - p1;

            let c0 = p0;
            let c2 = (p2 - p0 + tmp.mul_by_x()) * sp.gf2_deg2_eval_coef;
            let c1 = tmp - c2;
            c0 + (c2 * x + c1) * x
        } else {
            let c0 = p0;
            let c2 = C::ChallengeField::INV_2 * (p2 - p1 - p1 + p0);
            let c1 = p1 - p0 - c2;
            c0 + (c2 * x + c1) * x
        }
    }

    #[inline(always)]
    pub fn degree_3_eval<C: GKRFieldConfig>(
        vals: &[C::ChallengeField],
        x: C::ChallengeField,
        sp: &VerifierScratchPad<C>,
    ) -> C::ChallengeField {
        Self::lag_eval(vals, x, sp)
    }

    #[inline(always)]
    pub fn degree_6_eval<C: GKRFieldConfig>(
        vals: &[C::ChallengeField],
        x: C::ChallengeField,
        sp: &VerifierScratchPad<C>,
    ) -> C::ChallengeField {
        Self::lag_eval(vals, x, sp)
    }

    #[inline(always)]
    #[allow(clippy::needless_range_loop)]
    fn lag_eval<C: GKRFieldConfig>(
        vals: &[C::ChallengeField],
        x: C::ChallengeField,
        sp: &VerifierScratchPad<C>,
    ) -> C::ChallengeField {
        let (evals, lag_denoms_inv) = match vals.len() {
            4 => (sp.deg3_eval_at.to_vec(), sp.deg3_lag_denoms_inv.to_vec()),
            7 => (sp.deg6_eval_at.to_vec(), sp.deg6_lag_denoms_inv.to_vec()),
            _ => panic!("unsupported degree"),
        };

        let mut v = C::ChallengeField::ZERO;
        for i in 0..vals.len() {
            let mut numerator = C::ChallengeField::ONE;
            for j in 0..vals.len() {
                if j == i {
                    continue;
                }
                numerator *= x - evals[j];
            }
            v += numerator * lag_denoms_inv[i] * vals[i];
        }
        v
    }
}
