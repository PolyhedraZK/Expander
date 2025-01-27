mod common;

use arith::{BN254Fr, Field};
use gkr_field_config::{BN254Config, GF2ExtConfig, GKRFieldConfig, M31ExtConfig};
use mpi_config::MPIConfig;
use poly_commit::{
    raw::{RawExpanderGKR, RawExpanderGKRParams, RawMultiLinearPCS, RawMultiLinearParams},
    ExpanderGKRChallenge,
};
use polynomials::{MultiLinearPoly, RefMultiLinearPoly};
use rand::thread_rng;
use transcript::{BytesHashTranscript, Keccak256hasher, SHA256hasher, Transcript};

#[test]
fn test_raw() {
    let params = RawMultiLinearParams { n_vars: 8 };
    let mut rng = thread_rng();
    let poly = MultiLinearPoly::random(params.n_vars, &mut rng);
    let xs = (0..100)
        .map(|_| {
            (0..params.n_vars)
                .map(|_| BN254Fr::random_unsafe(&mut rng))
                .collect::<Vec<BN254Fr>>()
        })
        .collect::<Vec<Vec<BN254Fr>>>();

    common::test_pcs::<BN254Fr, BytesHashTranscript<_, Keccak256hasher>, RawMultiLinearPCS>(
        &params, &poly, &xs,
    );
}

fn test_raw_gkr_helper<C: GKRFieldConfig, T: Transcript<C::ChallengeField>>(
    mpi_config: &MPIConfig,
    transcript: &mut T,
) {
    let params = RawExpanderGKRParams { n_local_vars: 8 };
    let mut rng = thread_rng();
    let hypercube_basis = (0..(1 << params.n_local_vars))
        .map(|_| C::SimdCircuitField::random_unsafe(&mut rng))
        .collect();
    let poly = RefMultiLinearPoly::from_ref(&hypercube_basis);
    let xs = (0..100)
        .map(|_| ExpanderGKRChallenge::<C> {
            x: (0..params.n_local_vars)
                .map(|_| C::ChallengeField::random_unsafe(&mut rng))
                .collect::<Vec<C::ChallengeField>>(),
            x_simd: (0..C::get_field_pack_size().trailing_zeros())
                .map(|_| C::ChallengeField::random_unsafe(&mut rng))
                .collect::<Vec<C::ChallengeField>>(),
            x_mpi: (0..mpi_config.world_size().trailing_zeros())
                .map(|_| C::ChallengeField::random_unsafe(&mut rng))
                .collect::<Vec<C::ChallengeField>>(),
        })
        .collect::<Vec<ExpanderGKRChallenge<C>>>();
    common::test_pcs_for_expander_gkr::<C, T, RawExpanderGKR<C, T>>(
        &params, mpi_config, transcript, &poly, &xs,
    );
}

#[test]
fn test_raw_gkr() {
    let mpi_config = MPIConfig::new();

    type TM31 = BytesHashTranscript<<M31ExtConfig as GKRFieldConfig>::ChallengeField, SHA256hasher>;
    test_raw_gkr_helper::<M31ExtConfig, TM31>(&mpi_config, &mut TM31::new());

    type TGF2 = BytesHashTranscript<<GF2ExtConfig as GKRFieldConfig>::ChallengeField, SHA256hasher>;
    test_raw_gkr_helper::<GF2ExtConfig, TGF2>(&mpi_config, &mut TGF2::new());

    type TBN254 =
        BytesHashTranscript<<BN254Config as GKRFieldConfig>::ChallengeField, SHA256hasher>;
    test_raw_gkr_helper::<BN254Config, TBN254>(&mpi_config, &mut TBN254::new());

    MPIConfig::finalize();
}
