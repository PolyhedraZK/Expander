mod common;

use std::sync::Arc;

use arith::{BN254Fr, Field};
use common::test_gkr_pcs;
use gkr_field_config::{BN254Config, GF2ExtConfig, GKRFieldConfig, M31ExtConfig};
use mpi_config::MPIConfig;
use poly_commit::raw::{RawExpanderGKR, RawMultiLinearPCS, RawMultiLinearParams};
use polynomials::MultiLinearPoly;
use rand::thread_rng;
use serial_test::serial;
use transcript::{BytesHashTranscript, SHA256hasher};

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

    common::test_pcs::<BN254Fr, RawMultiLinearPCS>(&params, &poly, &xs);
}

#[test]
#[serial]
fn test_raw_gkr() {
    let global_data: Arc<[u8]> = Arc::from((0..1024).map(|i| i as u8).collect::<Vec<u8>>());
    let num_threads = rayon::current_num_threads();

    // Create configs for all threads
    let mpi_config = MPIConfig::new(num_threads as i32, global_data, 1024 * 1024);

    type TM31 = BytesHashTranscript<<M31ExtConfig as GKRFieldConfig>::ChallengeField, SHA256hasher>;
    test_gkr_pcs::<M31ExtConfig, TM31, RawExpanderGKR<_, _>>(&mpi_config, 8);

    type TGF2 = BytesHashTranscript<<GF2ExtConfig as GKRFieldConfig>::ChallengeField, SHA256hasher>;
    test_gkr_pcs::<GF2ExtConfig, TGF2, RawExpanderGKR<_, _>>(&mpi_config, 8);

    type TBN254 =
        BytesHashTranscript<<BN254Config as GKRFieldConfig>::ChallengeField, SHA256hasher>;
    test_gkr_pcs::<BN254Config, TBN254, RawExpanderGKR<_, _>>(&mpi_config, 8);

    mpi_config.finalize();
}
