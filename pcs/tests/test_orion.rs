use std::ops::Mul;

use arith::{Field, FieldSerde, SimdField};
use ark_std::test_rng;
use gf2::{GF2x128, GF2x8, GF2};
use gf2_128::{GF2_128x8, GF2_128};
use pcs::{OrionPCS, OrionPCSSetup, ORION_CODE_PARAMETER_INSTANCE};
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};

mod common;

fn test_orion_pcs_e2e_generics<F, EvalF, ComPackF, IPPackF, IPPackEvalF, T>(num_vars: usize)
where
    F: Field + FieldSerde,
    EvalF: Field + FieldSerde + Mul<F, Output = EvalF> + From<F>,
    ComPackF: SimdField<Scalar = F>,
    IPPackF: SimdField<Scalar = F>,
    IPPackEvalF: SimdField<Scalar = EvalF> + Mul<IPPackF, Output = IPPackEvalF>,
    T: Transcript<EvalF>,
{
    let params = OrionPCSSetup {
        num_vars,
        code_parameter: ORION_CODE_PARAMETER_INSTANCE,
    };

    let mut rng = test_rng();
    let poly = MultiLinearPoly::<F>::random(params.num_vars, &mut rng);

    (0..5).for_each(|_| {
        let opening_point: Vec<_> = (0..params.num_vars)
            .map(|_| EvalF::random_unsafe(&mut rng))
            .collect();

        common::test_pcs_e2e::<OrionPCS<F, EvalF, ComPackF, IPPackF, IPPackEvalF, T>>(
            &params,
            &poly,
            &opening_point,
            &mut rng,
        );
    })
}

#[test]
fn test_orion_pcs_e2e() {
    (19..=25).for_each(
        test_orion_pcs_e2e_generics::<
            GF2,
            GF2_128,
            GF2x128,
            GF2x8,
            GF2_128x8,
            BytesHashTranscript<_, Keccak256hasher>,
        >,
    );
}
