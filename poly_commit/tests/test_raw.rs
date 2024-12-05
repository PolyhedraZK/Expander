mod common;

use arith::{BN254Fr, Field};
use communicator::{ExpanderComm, MPICommunicator};
use gkr_field_config::{BN254Config, GF2ExtConfig, GKRFieldConfig, M31ExtConfig};
use poly_commit::{
    raw::{RawExpanderGKR, RawExpanderGKRParams, RawMultiLinear, RawMultiLinearParams},
    ExpanderGKRChallenge,
};
use polynomials::MultiLinearPoly;
use rand::thread_rng;
use transcript::{BytesHashTranscript, SHA256hasher, Transcript};

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

fn test_raw_gkr_helper<C: GKRFieldConfig, T: Transcript<C::ChallengeField>>(
    mpi_comm: &MPICommunicator,
    transcript: &mut T,
) {
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
            x_mpi: (0..mpi_comm.world_size().trailing_zeros())
                .map(|_| C::ChallengeField::random_unsafe(&mut rng))
                .collect::<Vec<C::ChallengeField>>(),
        })
        .collect::<Vec<ExpanderGKRChallenge<C>>>();
    common::test_gkr_pcs::<C, T, RawExpanderGKR<C, T>>(&params, mpi_comm, transcript, &poly, &xs);
}

#[test]
fn test_raw_gkr() {
    let mpi_comm = MPICommunicator::new(1);

    type TM31 = BytesHashTranscript<<M31ExtConfig as GKRFieldConfig>::ChallengeField, SHA256hasher>;
    test_raw_gkr_helper::<M31ExtConfig, TM31>(&mpi_comm, &mut TM31::new());

    type TGF2 = BytesHashTranscript<<GF2ExtConfig as GKRFieldConfig>::ChallengeField, SHA256hasher>;
    test_raw_gkr_helper::<GF2ExtConfig, TGF2>(&mpi_comm, &mut TGF2::new());

    type TBN254 =
        BytesHashTranscript<<BN254Config as GKRFieldConfig>::ChallengeField, SHA256hasher>;
    test_raw_gkr_helper::<BN254Config, TBN254>(&mpi_comm, &mut TBN254::new());

    MPICommunicator::finalize();
}
