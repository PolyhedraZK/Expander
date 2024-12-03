mod common;

use arith::{ExtensionField, Field, SimdField};
use ark_std::test_rng;
use gf2::{GF2x128, GF2x64, GF2x8, GF2};
use gf2_128::GF2_128;
use gkr_field_config::{GF2ExtConfig, GKRFieldConfig};
use mpi_config::MPIConfig;
use poly_commit::*;
use polynomials::MultiLinearPoly;
use raw::RawExpanderGKR;
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
    let srs =
        <OrionSIMDFieldPCS<
            C::CircuitField,
            C::SimdCircuitField,
            C::ChallengeField,
            ComPackF,
            OpenPackF,
            T,
        > as PCSForExpanderGKR<C, T>>::gen_srs_for_testing(&num_vars, &mpi_config, &mut rng);
    let (pk, vk) = srs.into_keys();

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

    let mut scratch_pad =
        <OrionSIMDFieldPCS<
            C::CircuitField,
            C::SimdCircuitField,
            C::ChallengeField,
            ComPackF,
            OpenPackF,
            T,
        > as PCSForExpanderGKR<C, T>>::init_scratch_pad(&num_vars, &mpi_config);

    let mut local_prover_transcript = T::new();
    let mut local_verifier_transcript = T::new();

    let expected_global_eval = RawExpanderGKR::<C, T>::eval(
        &poly.coeffs,
        &challenge_point.x,
        &challenge_point.x_simd,
        &challenge_point.x_mpi,
    );

    dbg!(poly.get_num_vars(), poly.coeffs[0]);
    dbg!(pk.num_vars);
    dbg!(&challenge_point.x_mpi);
    dbg!(mpi_config.world_size(), mpi_config.world_rank());
    dbg!(expected_global_eval);

    // NOTE separate polynomial into different pieces by mpi rank
    let poly_vars_stride = (1 << poly.get_num_vars()) / mpi_config.world_size();
    let poly_coeff_starts = mpi_config.world_rank() * poly_vars_stride;
    let poly_coeff_ends = poly_coeff_starts + poly_vars_stride;
    let local_poly = MultiLinearPoly::new(poly.coeffs[poly_coeff_starts..poly_coeff_ends].to_vec());

    let expected_local_eval = RawExpanderGKR::<C, T>::eval_local(
        &local_poly.coeffs,
        &challenge_point.x,
        &challenge_point.x_simd,
    );

    dbg!(local_poly.get_num_vars(), local_poly.coeffs[0]);

    // NOTE commit polynomial in different parts
    let commitment = <OrionSIMDFieldPCS<
        C::CircuitField,
        C::SimdCircuitField,
        C::ChallengeField,
        ComPackF,
        OpenPackF,
        T,
    > as PCSForExpanderGKR<C, T>>::commit(
        &num_vars,
        &mpi_config,
        &pk,
        &local_poly,
        &mut scratch_pad,
    );
    dbg!(commitment);

    // NOTE: open polynomial in different parts
    let opening = <OrionSIMDFieldPCS<
        C::CircuitField,
        C::SimdCircuitField,
        C::ChallengeField,
        ComPackF,
        OpenPackF,
        T,
    > as PCSForExpanderGKR<C, T>>::open(
        &num_vars,
        &mpi_config,
        &pk,
        &local_poly,
        &challenge_point,
        &mut local_prover_transcript,
        &mut scratch_pad,
    );
    dbg!(opening.query_openings.len());

    // NOTE verify polynomial in different parts
    let pass = <OrionSIMDFieldPCS<
        C::CircuitField,
        C::SimdCircuitField,
        C::ChallengeField,
        ComPackF,
        OpenPackF,
        T,
    > as PCSForExpanderGKR<C, T>>::verify(
        &num_vars,
        &mpi_config,
        &vk,
        &commitment,
        &challenge_point,
        if mpi_config.is_root() {
            expected_global_eval
        } else {
            expected_local_eval
        },
        &mut local_verifier_transcript,
        &opening,
    );

    assert!(pass);

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
