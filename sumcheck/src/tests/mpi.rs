use circuit::Circuit;
use gkr_engine::{ExpanderDualVarChallenge, GKREngine, GKRScheme, M31x16Config, M31x1Config, MPIConfig, MPIEngine};
use gkr_hashers::SHA256hasher;
use poly_commit::RawExpanderGKR;
use transcript::BytesHashTranscript;

use crate::{prover_helper::SumcheckGkrVanillaHelper, SUMCHECK_GKR_DEGREE};

struct M31x16Sha2RawCudaDev;

impl GKREngine for M31x16Sha2RawCudaDev {
    type FieldConfig = M31x16Config;
    type MPIConfig = MPIConfig;
    type TranscriptConfig = BytesHashTranscript<SHA256hasher>;
    type PCSConfig = RawExpanderGKR<M31x16Config>;
    const SCHEME: GKRScheme = GKRScheme::Vanilla;
    const CUDA_DEV: bool = true;
}

struct M31x1Sha2RawCudaDev;

impl GKREngine for M31x1Sha2RawCudaDev {
    type FieldConfig = M31x1Config;
    type MPIConfig = MPIConfig;
    type TranscriptConfig = BytesHashTranscript<SHA256hasher>;
    type PCSConfig = RawExpanderGKR<M31x1Config>;
    const SCHEME: GKRScheme = GKRScheme::Vanilla;
    const CUDA_DEV: bool = true;
}

/// mpiexec -n 16 cargo test --release test_mpi_matches -- --nocapture
/// This tests checks that
/// - M31x1 with MPI = 16
/// - M31x16 with MPI = 1
/// generates the same transcript for poly_evals_at_rx function
#[test]
fn test_mpi_matches() {
    let mpi_config = MPIConfig::prover_new();

    // mpi = 1
    if mpi_config.is_root() {
        // load circuit
        let mut circuit_x16 =
            Circuit::<M31x16Config>::single_thread_prover_load_circuit::<M31x16Sha2RawCudaDev>("../data/circuit_m31.txt");
        let layers = circuit_x16.layers[1];

        let challenge = ExpanderDualVarChallenge::default();


        let mut helper = SumcheckGkrVanillaHelper::new(
            layer,
            &challenge,
            alpha,
            sp,
            &mpi_config,
            false,
        );
        let evals = helper.poly_evals_at_rx(i_var, SUMCHECK_GKR_DEGREE, &mpi_config);
    }

    assert!(false, "This test is not implemented")
}
