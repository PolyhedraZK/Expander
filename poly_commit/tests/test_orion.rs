mod common;

use arith::{ExtensionField, Field, SimdField};
use ark_std::test_rng;
use gf2::{GF2x128, GF2x64, GF2x8, GF2};
use gf2_128::GF2_128;
use mersenne31::{M31Ext3, M31x16, M31};
use poly_commit::*;
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher};

const TEST_REPETITION: usize = 3;

fn test_orion_base_field_pcs_generics<F, EvalF, ComPackF, OpenPackF>(
    num_vars_start: usize,
    num_vars_end: usize,
) where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
{
    let mut rng = test_rng();

    (num_vars_start..=num_vars_end).for_each(|num_vars| {
        let xs: Vec<_> = (0..TEST_REPETITION)
            .map(|_| -> Vec<EvalF> {
                (0..num_vars)
                    .map(|_| EvalF::random_unsafe(&mut rng))
                    .collect()
            })
            .collect();
        let poly = MultiLinearPoly::<F>::random(num_vars, &mut rng);

        common::test_pcs::<
            EvalF,
            BytesHashTranscript<EvalF, Keccak256hasher>,
            OrionBaseFieldPCS<
                F,
                EvalF,
                ComPackF,
                OpenPackF,
                BytesHashTranscript<EvalF, Keccak256hasher>,
            >,
        >(&num_vars, &poly, &xs);
    })
}

#[test]
fn test_orion_base_field_pcs_full_e2e() {
    test_orion_base_field_pcs_generics::<GF2, GF2_128, GF2x64, GF2x8>(19, 25);
    test_orion_base_field_pcs_generics::<GF2, GF2_128, GF2x128, GF2x8>(19, 25);
    test_orion_base_field_pcs_generics::<M31, M31Ext3, M31x16, M31x16>(16, 22)
}

fn test_orion_simd_field_pcs_generics<F, SimdF, EvalF, ComPackF>(
    num_vars_start: usize,
    num_vars_end: usize,
) where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let mut rng = test_rng();

    (num_vars_start..=num_vars_end).for_each(|num_vars| {
        let poly_num_vars = num_vars - SimdF::PACK_SIZE.ilog2() as usize;
        let xs: Vec<_> = (0..TEST_REPETITION)
            .map(|_| -> Vec<EvalF> {
                (0..num_vars)
                    .map(|_| EvalF::random_unsafe(&mut rng))
                    .collect()
            })
            .collect();
        let poly = MultiLinearPoly::<SimdF>::random(poly_num_vars, &mut rng);

        common::test_pcs::<
            EvalF,
            BytesHashTranscript<EvalF, Keccak256hasher>,
            OrionSIMDFieldPCS<
                F,
                SimdF,
                EvalF,
                ComPackF,
                BytesHashTranscript<EvalF, Keccak256hasher>,
            >,
        >(&num_vars, &poly, &xs);
    })
}

#[test]
fn test_orion_simd_field_pcs_full_e2e() {
    test_orion_simd_field_pcs_generics::<GF2, GF2x8, GF2_128, GF2x64>(19, 25);
    test_orion_simd_field_pcs_generics::<GF2, GF2x8, GF2_128, GF2x128>(19, 25);
    test_orion_simd_field_pcs_generics::<M31, M31x16, M31Ext3, M31x16>(16, 22);
}
