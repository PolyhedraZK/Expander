use arith::Field;
use ark_std::{rand::Rng, test_rng};
use config::BN254ConfigSha2;
use halo2curves::bn256::Fr;
use polynomials::MultiLinearPoly;

use crate::prover_helper::SumcheckProductGateHelper;

#[test]
fn sumcheck_prove_gkr_layer() {
    sumcheck_prove_gkr_layer_helper::<Fr>(10);
}

fn sumcheck_prove_gkr_layer_helper<F: Field>(nv: usize) {
    let mut rng = test_rng();

    let point = (0..nv).map(|_| F::random_unsafe(&mut rng)).collect::<Vec<_>>();
    let mut v_evals = MultiLinearPoly::random(nv, &mut rng).coeffs;
    let mut hg_evals = MultiLinearPoly::random(nv, &mut rng).coeffs;
    let mut input_vals = MultiLinearPoly::random(3, &mut rng).coeffs;
    let mut gate_exists_5 = (0..100).map(|_| rng.gen_bool(0.1)).collect::<Vec<_>>();
    gate_exists_5.resize(1 << nv, false);

    for cur_nv in 0..nv {
        let xy_helper = SumcheckProductGateHelper::new(cur_nv);

        let local_vals_simd = xy_helper.poly_eval_at::<BN254ConfigSha2>(
            cur_nv,
            2,
            &v_evals,
            &hg_evals,
            &input_vals,
            &gate_exists_5,
        );
    }
}
