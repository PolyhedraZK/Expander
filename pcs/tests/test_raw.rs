mod common;

use arith::{BN254Fr, Field};
use ark_std::test_rng;
use pcs::raw::{RawMultilinearPCS, RawMultilinearPCSPublicParams};
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher};

#[test]
fn test_raw_pcs() {
    let params = RawMultilinearPCSPublicParams { n_vars: 8 };
    let mut rng = test_rng();
    let poly = MultiLinearPoly::random(params.n_vars, &mut rng);

    (0..100).for_each(|_| {
        let opening_point = (0..params.n_vars)
            .map(|_| BN254Fr::random_unsafe(&mut rng))
            .collect();

        common::test_pcs_e2e::<
            BN254Fr,
            RawMultilinearPCS<_, BytesHashTranscript<_, Keccak256hasher>>,
        >(&params, &poly, &opening_point, &mut rng);
    });
}