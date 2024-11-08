mod common;

use std::marker::PhantomData;

use arith::{BN254Fr, Field};
use pcs::raw::{RawML, RawMLParams};
use polynomials::MultiLinearPoly;
use rand::thread_rng;

#[test]
fn test_raw() {
    let params = RawMLParams { n_vars: 8 };
    let mut raw_ml: RawML<BN254Fr> = RawML {
        _phantom: PhantomData,
    };
    let mut rng = thread_rng();
    let poly = MultiLinearPoly::random(params.n_vars, &mut rng);
    let xs = (0..100)
        .map(|_| {
            (0..params.n_vars)
                .map(|_| BN254Fr::random_unsafe(&mut rng))
                .collect::<Vec<BN254Fr>>()
        })
        .collect::<Vec<Vec<BN254Fr>>>();

    common::test_pcs::<BN254Fr, RawML<_>>(&mut raw_ml, &params, &poly, &xs);
}
