use arith::Field;
use ark_std::test_rng;
use gf2::{GF2x128, GF2x8, GF2};
use gf2_128::{GF2_128x8, GF2_128};
use pcs::{OrionPCS, OrionPCSSetup, ORION_CODE_PARAMETER_INSTANCE};
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher};

mod common;

#[test]
fn test_orion_pcs() {
    let params = OrionPCSSetup {
        num_vars: 22,
        code_parameter: ORION_CODE_PARAMETER_INSTANCE,
    };

    let mut rng = test_rng();
    let poly = MultiLinearPoly::<GF2>::random(params.num_vars, &mut rng);

    (0..100).for_each(|_| {
        let opening_point: Vec<_> = (0..params.num_vars)
            .map(|_| GF2_128::random_unsafe(&mut rng))
            .collect();

        common::test_pcs_e2e::<
            OrionPCS<
                GF2,
                GF2x128,
                GF2_128,
                GF2x8,
                GF2_128x8,
                BytesHashTranscript<_, Keccak256hasher>,
            >,
        >(&params, &poly, &opening_point, &mut rng);
    })
}
