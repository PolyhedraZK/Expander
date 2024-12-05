mod common;

use arith::{ExtensionField, Field, SimdField};
use ark_std::test_rng;
use gf2::{GF2x128, GF2x64, GF2x8, GF2};
use gf2_128::GF2_128;
use gkr_field_config::{GF2ExtConfig, GKRFieldConfig};
use mpi_config::MPIConfig;
use poly_commit::*;
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};

const TEST_REPETITION: usize = 3;

fn test_orion_base_field_pcs_generics<F, EvalF, ComPackF, OpenPackF>()
where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
{
    let mut rng = test_rng();

    (19..=25).for_each(|num_vars| {
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
    test_orion_base_field_pcs_generics::<GF2, GF2_128, GF2x64, GF2x8>();
    test_orion_base_field_pcs_generics::<GF2, GF2_128, GF2x128, GF2x8>()
}

fn test_orion_simd_field_pcs_generics<F, SimdF, EvalF, ComPackF, OpenPackF>()
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
{
    let mut rng = test_rng();

    (19..=25).for_each(|num_vars| {
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
                OpenPackF,
                BytesHashTranscript<EvalF, Keccak256hasher>,
            >,
        >(&num_vars, &poly, &xs);
    })
}

#[test]
fn test_orion_simd_field_pcs_full_e2e() {
    test_orion_simd_field_pcs_generics::<GF2, GF2x8, GF2_128, GF2x64, GF2x8>();
    test_orion_simd_field_pcs_generics::<GF2, GF2x8, GF2_128, GF2x128, GF2x8>();
}

fn test_orion_for_expander_gkr_generics<C, ComPackF, OpenPackF, T>(num_vars: usize)
where
    C: GKRFieldConfig,
    ComPackF: SimdField<Scalar = C::CircuitField>,
    OpenPackF: SimdField<Scalar = C::CircuitField>,
    T: Transcript<C::ChallengeField>,
{
    let mut rng = test_rng();
    let mpi_config = MPIConfig::new();

    // NOTE: generate global random polynomial
    let num_vars_in_simd = C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
    let num_vars_in_mpi = mpi_config.world_size().ilog2() as usize;
    let poly =
        MultiLinearPoly::<C::SimdCircuitField>::random(num_vars - num_vars_in_simd, &mut rng);

    // NOTE generate srs for each party, and shared challenge point in each party
    let challenge_point = ExpanderGKRChallenge::<C> {
        x_mpi: (0..num_vars_in_mpi)
            .map(|_| C::ChallengeField::random_unsafe(&mut rng))
            .collect(),
        x_simd: (0..num_vars_in_simd)
            .map(|_| C::ChallengeField::random_unsafe(&mut rng))
            .collect(),
        x: (0..(num_vars - num_vars_in_mpi - num_vars_in_simd))
            .map(|_| C::ChallengeField::random_unsafe(&mut rng))
            .collect(),
    };

    let mut transcript = T::new();

    dbg!(poly.get_num_vars(), poly.coeffs[0]);
    dbg!(&challenge_point.x_mpi);
    dbg!(mpi_config.world_size(), mpi_config.world_rank());

    // NOTE separate polynomial into different pieces by mpi rank
    let poly_vars_stride = (1 << poly.get_num_vars()) / mpi_config.world_size();
    let poly_coeff_starts = mpi_config.world_rank() * poly_vars_stride;
    let poly_coeff_ends = poly_coeff_starts + poly_vars_stride;
    let local_poly = MultiLinearPoly::new(poly.coeffs[poly_coeff_starts..poly_coeff_ends].to_vec());

    dbg!(local_poly.get_num_vars(), local_poly.coeffs[0]);

    common::test_pcs_for_expander_gkr::<
        C,
        T,
        OrionSIMDFieldPCS<
            C::CircuitField,
            C::SimdCircuitField,
            C::ChallengeField,
            ComPackF,
            OpenPackF,
            T,
        >,
    >(
        &num_vars,
        &mpi_config,
        &mut transcript,
        &local_poly,
        &vec![challenge_point],
    );

    MPIConfig::finalize()
}

#[test]
fn test_orion_for_expander_gkr() {
    test_orion_for_expander_gkr_generics::<
        GF2ExtConfig,
        GF2x128,
        GF2x8,
        BytesHashTranscript<_, Keccak256hasher>,
    >(30);
}
