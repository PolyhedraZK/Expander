mod common;

use arith::{BN254Fr, Field};
use gkr_field_config::{BN254Config, GF2ExtConfig, GKRFieldConfig, M31ExtConfig};
use mpi_config::MPIConfig;
use polynomial_commitment_scheme::{
    raw::{RawExpanderGKR, RawExpanderGKRParams, RawMultiLinear, RawMultiLinearParams},
    ExpanderGKRChallenge,
};
use polynomials::MultiLinearPoly;
use rand::thread_rng;

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

    common::test_pcs::<BN254Fr, RawMultiLinear>(&params, &poly, &xs);
}

fn test_raw_gkr_helper<C: GKRFieldConfig>(mpi_config: &MPIConfig) {
    let params = RawExpanderGKRParams { n_local_vars: 8 };
    let mut rng = thread_rng();
    let poly = MultiLinearPoly::<C::SimdCircuitField>::random(params.n_local_vars, &mut rng);
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
    common::test_gkr_pcs::<C, RawExpanderGKR<C>>(&params, mpi_config, &poly, &xs);
}

#[test]
fn test_raw_gkr() {
    let mpi_config = MPIConfig::new();
    test_raw_gkr_helper::<M31ExtConfig>(&mpi_config);
    test_raw_gkr_helper::<GF2ExtConfig>(&mpi_config);
    test_raw_gkr_helper::<BN254Config>(&mpi_config);
    MPIConfig::finalize();
}
