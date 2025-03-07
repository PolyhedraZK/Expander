use arith::FieldSerde;
use circuit::Circuit;
use config::{FiatShamirHashType, GKRConfig, PolynomialCommitmentType};
use config_macros::declare_gkr_config;
use gkr_field_config::{BN254Config, FieldType, GF2ExtConfig, GKRFieldConfig, M31ExtConfig};
use mpi_config::{root_println, shared_mem::SharedMemory, MPIConfig};
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

fn load_circuit<Cfg: GKRConfig>(mpi_config: &MPIConfig) -> Option<Circuit<Cfg::FieldConfig>> {
    let circuit_path = match <Cfg as GKRConfig>::FieldConfig::FIELD_TYPE {
        FieldType::GF2 => "../".to_owned() + KECCAK_GF2_CIRCUIT,
        FieldType::M31 => "../".to_owned() + KECCAK_M31_CIRCUIT,
        FieldType::BN254 => "../".to_owned() + KECCAK_BN254_CIRCUIT,
        _ => unreachable!(),
    };
    
    if mpi_config.is_root() {
        Some(Circuit::<Cfg::FieldConfig>::load_circuit::<Cfg>(&circuit_path))
    } else {
        None
    }
}

#[test]
fn test_shared_mem() {
    let mpi_config = MPIConfig::new();
    test_shared_mem_helper(&mpi_config, 123u8);
    test_shared_mem_helper(&mpi_config, 456789usize);
    test_shared_mem_helper(&mpi_config, vec![1usize, 2, 3]);
    MPIConfig::finalize();
}

#[allow(unreachable_patterns)]
fn test_shared_mem_helper<T: SharedMemory+FieldSerde>(mpi_config: &MPIConfig, t: T) {
    let mut original_serialization = vec![];
    let (data, mut window) = if mpi_config.is_root() {
        t.serialize_into(&mut original_serialization).unwrap();
        mpi_config.consume_obj_and_create_shared(Some(t))
    } else {
        mpi_config.consume_obj_and_create_shared(None)
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
        gathered_bytes.chunks_exact_mut(original_serialization.len()).for_each(|chunk| {
            assert_eq!(chunk, &original_serialization[..]);
        });
    }
    mpi_config.free_shared_mem(&mut window);
}
