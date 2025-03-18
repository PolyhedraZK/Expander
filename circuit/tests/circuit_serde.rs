use arith::FieldSerde;
use circuit::Circuit;
use config::{FiatShamirHashType, GKRConfig, PolynomialCommitmentType};
use config_macros::declare_gkr_config;
use gkr_field_config::{BN254Config, FieldType, GF2ExtConfig, GKRFieldConfig, M31ExtConfig};
use mpi_config::{root_println, MPIConfig};
use poly_commit::RawExpanderGKR;
use transcript::{BytesHashTranscript, SHA256hasher};

// circuit for repeating Keccak for 2 times
pub const KECCAK_M31_CIRCUIT: &str = "data/circuit_m31.txt";
pub const KECCAK_GF2_CIRCUIT: &str = "data/circuit_gf2.txt";
pub const KECCAK_BN254_CIRCUIT: &str = "data/circuit_bn254.txt";

declare_gkr_config!(
    pub M31ExtConfigSha2Raw,
    FieldType::M31,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw
);

declare_gkr_config!(
    pub GF2ExtConfigSha2Raw,
    FieldType::GF2,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw
);

declare_gkr_config!(
    pub BN254ConfigSha2Raw,
    FieldType::BN254,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw
);

#[test]
fn test_circuit_serde() {
    let mpi_config = MPIConfig::new();
    test_circuit_serde_helper::<M31ExtConfigSha2Raw>(&mpi_config);
    test_circuit_serde_helper::<GF2ExtConfigSha2Raw>(&mpi_config);
    test_circuit_serde_helper::<BN254ConfigSha2Raw>(&mpi_config);
    MPIConfig::finalize();
}

#[allow(unreachable_patterns)]
fn test_circuit_serde_helper<Cfg: GKRConfig>(mpi_config: &MPIConfig) {
    root_println!(
        mpi_config,
        "Field Type: {:?}",
        <Cfg as GKRConfig>::FieldConfig::FIELD_TYPE
    );

    let circuit_path = match <Cfg as GKRConfig>::FieldConfig::FIELD_TYPE {
        FieldType::GF2 => "../".to_owned() + KECCAK_GF2_CIRCUIT,
        FieldType::M31 => "../".to_owned() + KECCAK_M31_CIRCUIT,
        FieldType::BN254 => "../".to_owned() + KECCAK_BN254_CIRCUIT,
        _ => unreachable!(),
    };
    let circuit =
        Circuit::<Cfg::FieldConfig>::single_thread_prover_load_circuit::<Cfg>(&circuit_path);
    root_println!(mpi_config, "Circuit loaded.");

    let mut buffer = vec![];
    circuit.serialize_into(&mut buffer).unwrap();
    let circuit_deserialized = Circuit::<Cfg::FieldConfig>::deserialize_from(&buffer[..]).unwrap();

    let mut buffer2 = vec![];
    circuit_deserialized.serialize_into(&mut buffer2).unwrap();
    assert_eq!(buffer, buffer2);
}
