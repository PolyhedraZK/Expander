use circuit::Circuit;
use config_macros::declare_gkr_config;
use gkr_engine::{
    BN254Config, FieldEngine, FieldType, GF2ExtConfig, GKREngine, GKRScheme, GoldilocksExtConfig,
    M31ExtConfig, MPIConfig, MPIEngine, SharedMemory,
};
use gkr_hashers::SHA256hasher;
use poly_commit::RawExpanderGKR;
use serdes::ExpSerde;
use transcript::BytesHashTranscript;

// circuit for repeating Keccak for 2 times
pub const KECCAK_M31_CIRCUIT: &str = "data/circuit_m31.txt";
pub const KECCAK_GF2_CIRCUIT: &str = "data/circuit_gf2.txt";
pub const KECCAK_BN254_CIRCUIT: &str = "data/circuit_bn254.txt";
pub const KECCAK_GOLDILOCKS_CIRCUIT: &str = "data/circuit_goldilocks.txt";

declare_gkr_config!(
    M31ExtConfigSha2Raw,
    FieldType::M31Ext3,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);

declare_gkr_config!(
    GF2ExtConfigSha2Raw,
    FieldType::GF2Ext128,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);

declare_gkr_config!(
    BN254ConfigSha2Raw,
    FieldType::BN254,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);

declare_gkr_config!(
    GoldilocksExtConfigSha2Raw,
    FieldType::GoldilocksExt2,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);

#[allow(unreachable_patterns)]
fn load_circuit<Cfg: GKREngine>(mpi_config: &MPIConfig) -> Option<Circuit<Cfg::FieldConfig>> {
    let circuit_path = match <Cfg as GKREngine>::FieldConfig::FIELD_TYPE {
        FieldType::GF2Ext128 => "../".to_owned() + KECCAK_GF2_CIRCUIT,
        FieldType::M31Ext3 => "../".to_owned() + KECCAK_M31_CIRCUIT,
        FieldType::BN254 => "../".to_owned() + KECCAK_BN254_CIRCUIT,
        FieldType::GoldilocksExt2 => "../".to_owned() + KECCAK_GOLDILOCKS_CIRCUIT,
        _ => unreachable!(),
    };

    if mpi_config.is_root() {
        Some(Circuit::<Cfg::FieldConfig>::single_thread_prover_load_circuit::<Cfg>(&circuit_path))
    } else {
        None
    }
}

#[test]
fn test_shared_mem() {
    let mpi_config = MPIConfig::prover_new();
    test_shared_mem_helper(&mpi_config, Some(123u8));
    test_shared_mem_helper(&mpi_config, Some(456789usize));
    test_shared_mem_helper(&mpi_config, Some(vec![1u8, 2, 3]));
    test_shared_mem_helper(&mpi_config, Some(vec![4usize, 5, 6]));
    test_shared_mem_helper(&mpi_config, Some((7u8, 8usize)));
    test_shared_mem_helper(&mpi_config, Some((9usize, 10u8)));

    let circuit = load_circuit::<M31ExtConfigSha2Raw>(&mpi_config);
    test_shared_mem_helper(&mpi_config, circuit);
    let circuit = load_circuit::<GF2ExtConfigSha2Raw>(&mpi_config);
    test_shared_mem_helper(&mpi_config, circuit);
    let circuit = load_circuit::<BN254ConfigSha2Raw>(&mpi_config);
    test_shared_mem_helper(&mpi_config, circuit);
    let circuit = load_circuit::<GoldilocksExtConfigSha2Raw>(&mpi_config);
    test_shared_mem_helper(&mpi_config, circuit);

    MPIConfig::finalize();
}

#[allow(unreachable_patterns)]
fn test_shared_mem_helper<T: SharedMemory + ExpSerde + std::fmt::Debug>(
    mpi_config: &MPIConfig,
    t: Option<T>,
) {
    let mut original_serialization = vec![];
    let (data, mut window) = if mpi_config.is_root() {
        t.as_ref()
            .unwrap()
            .serialize_into(&mut original_serialization)
            .unwrap();
        mpi_config.consume_obj_and_create_shared(t)
    } else {
        mpi_config.consume_obj_and_create_shared(t)
    };

    let mut shared_serialization = vec![];
    data.serialize_into(&mut shared_serialization).unwrap();

    let mut gathered_bytes = if mpi_config.is_root() {
        vec![0u8; original_serialization.len() * mpi_config.world_size()]
    } else {
        vec![]
    };
    mpi_config.gather_vec(&shared_serialization, &mut gathered_bytes);
    if mpi_config.is_root() {
        gathered_bytes
            .chunks_exact_mut(original_serialization.len())
            .enumerate()
            .for_each(|(i, chunk)| {
                assert_eq!(
                    chunk,
                    &original_serialization[..],
                    "rank {} not consistent",
                    i
                );
            });
    }
    data.discard_control_of_shared_mem();
    mpi_config.free_shared_mem(&mut window);
}
