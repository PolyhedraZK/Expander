mod common;

use arith::{ExtensionField, Field, SimdField};
use ark_std::test_rng;
use gf2::{GF2x128, GF2x64, GF2x8, GF2};
use gf2_128::GF2_128;
use gkr_field_config::{GF2ExtConfig, GKRFieldConfig, M31ExtConfig};
use mersenne31::{M31Ext3, M31x16, M31};
use mpi_config::MPIConfig;
use poly_commit::*;
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};

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

fn test_orion_for_expander_gkr_generics<C, ComPackF, T>(
    mpi_config_ref: &MPIConfig,
    total_num_vars: usize,
) where
    C: GKRFieldConfig,
    ComPackF: SimdField<Scalar = C::CircuitField>,
    T: Transcript<C::ChallengeField>,
{
    let mut rng = test_rng();

    // NOTE: generate global random polynomial
    let num_vars_in_simd = C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
    let num_vars_in_mpi = mpi_config_ref.world_size().ilog2() as usize;
    let num_vars_in_each_poly = total_num_vars - num_vars_in_mpi - num_vars_in_simd;
    let num_vars_in_global_poly = total_num_vars - num_vars_in_simd;

    let global_poly =
        MultiLinearPoly::<C::SimdCircuitField>::random(num_vars_in_global_poly, &mut rng);

    // NOTE generate srs for each party, and shared challenge point in each party
    let challenge_point = ExpanderGKRChallenge::<C> {
        x_mpi: (0..num_vars_in_mpi)
            .map(|_| C::ChallengeField::random_unsafe(&mut rng))
            .collect(),
        x_simd: (0..num_vars_in_simd)
            .map(|_| C::ChallengeField::random_unsafe(&mut rng))
            .collect(),
        x: (0..num_vars_in_each_poly)
            .map(|_| C::ChallengeField::random_unsafe(&mut rng))
            .collect(),
    };

    let mut transcript = T::new();

    dbg!(global_poly.get_num_vars(), global_poly.coeffs[0]);
    dbg!(&challenge_point.x_mpi);
    dbg!(mpi_config_ref.world_size(), mpi_config_ref.world_rank());

    // NOTE separate polynomial into different pieces by mpi rank
    let poly_vars_stride = (1 << global_poly.get_num_vars()) / mpi_config_ref.world_size();
    let poly_coeff_starts = mpi_config_ref.world_rank() * poly_vars_stride;
    let poly_coeff_ends = poly_coeff_starts + poly_vars_stride;
    let local_poly =
        MultiLinearPoly::new(global_poly.coeffs[poly_coeff_starts..poly_coeff_ends].to_vec());

    dbg!(local_poly.get_num_vars(), local_poly.coeffs[0]);

    common::test_pcs_for_expander_gkr::<
        C,
        T,
        OrionSIMDFieldPCS<C::CircuitField, C::SimdCircuitField, C::ChallengeField, ComPackF, T>,
    >(
        &num_vars_in_each_poly,
        mpi_config_ref,
        &mut transcript,
        &local_poly,
        &[challenge_point],
    );
}

#[test]
fn test_orion_for_expander_gkr() {
    let mpi_config = MPIConfig::new();

    test_orion_for_expander_gkr_generics::<
        GF2ExtConfig,
        GF2x128,
        BytesHashTranscript<_, Keccak256hasher>,
    >(&mpi_config, 16);

    test_orion_for_expander_gkr_generics::<
        M31ExtConfig,
        M31x16,
        BytesHashTranscript<_, Keccak256hasher>,
    >(&mpi_config, 15);

    MPIConfig::finalize()
}
