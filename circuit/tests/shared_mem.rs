use std::sync::{Arc, Mutex};

use arith::Field;
use ark_std::test_rng;
use circuit::Circuit;
use config_macros::declare_gkr_config;
use gkr_engine::{
    BN254Config, FieldEngine, FieldType, GF2ExtConfig, GKREngine, GKRScheme, Goldilocksx8Config,
    M31x16Config, MPIConfig, MPIEngine, MPISharedMemory,
};
use gkr_hashers::SHA256hasher;
use mersenne31::M31x16;
use poly_commit::RawExpanderGKR;
use serdes::ExpSerde;
use transcript::BytesHashTranscript;

// circuit for repeating Keccak for 2 times
pub const KECCAK_M31_CIRCUIT: &str = "data/circuit_m31.txt";
pub const KECCAK_GF2_CIRCUIT: &str = "data/circuit_gf2.txt";
pub const KECCAK_BN254_CIRCUIT: &str = "data/circuit_bn254.txt";
pub const KECCAK_GOLDILOCKS_CIRCUIT: &str = "data/circuit_goldilocks.txt";

declare_gkr_config!(
    M31x16ConfigSha2Raw,
    FieldType::M31x16,
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
    Goldilocksx8ConfigSha2Raw,
    FieldType::Goldilocksx8,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);

#[allow(unreachable_patterns)]
fn load_circuit<Cfg: GKREngine>(mpi_config: &MPIConfig) -> Option<Circuit<Cfg::FieldConfig>> {
    let circuit_path = match <Cfg as GKREngine>::FieldConfig::FIELD_TYPE {
        FieldType::GF2Ext128 => "../".to_owned() + KECCAK_GF2_CIRCUIT,
        FieldType::M31x16 => "../".to_owned() + KECCAK_M31_CIRCUIT,
        FieldType::BN254 => "../".to_owned() + KECCAK_BN254_CIRCUIT,
        FieldType::Goldilocksx8 => "../".to_owned() + KECCAK_GOLDILOCKS_CIRCUIT,
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
    let mut rng = test_rng();
    let universe = MPIConfig::init().unwrap();
    let world = universe.world();
    let mpi_config = MPIConfig::prover_new(Some(&universe), Some(&world));
    test_shared_mem_helper(&mpi_config, Some(456789usize));
    test_shared_mem_helper(&mpi_config, Some(vec![4usize, 5, 6]));
    test_shared_mem_helper(
        &mpi_config,
        Some(vec![
            M31x16::random_unsafe(&mut rng),
            M31x16::random_unsafe(&mut rng),
        ]),
    );

    test_shared_mem_on_heap_helper(
        &mpi_config,
        Some(vec![
            M31x16::random_unsafe(&mut rng),
            M31x16::random_unsafe(&mut rng),
        ]),
    );

    let circuit = load_circuit::<M31x16ConfigSha2Raw>(&mpi_config);
    test_shared_mem_helper(&mpi_config, circuit);
    let circuit = load_circuit::<GF2ExtConfigSha2Raw>(&mpi_config);
    test_shared_mem_helper(&mpi_config, circuit);
    let circuit = load_circuit::<BN254ConfigSha2Raw>(&mpi_config);
    test_shared_mem_helper(&mpi_config, circuit);
    let circuit = load_circuit::<Goldilocksx8ConfigSha2Raw>(&mpi_config);
    test_shared_mem_helper(&mpi_config, circuit);

    let circuit = load_circuit::<M31x16ConfigSha2Raw>(&mpi_config);
    test_shared_mem_on_heap_helper(&mpi_config, circuit);
    let circuit = load_circuit::<GF2ExtConfigSha2Raw>(&mpi_config);
    test_shared_mem_on_heap_helper(&mpi_config, circuit);
    let circuit = load_circuit::<BN254ConfigSha2Raw>(&mpi_config);
    test_shared_mem_on_heap_helper(&mpi_config, circuit);
    let circuit = load_circuit::<Goldilocksx8ConfigSha2Raw>(&mpi_config);
    test_shared_mem_on_heap_helper(&mpi_config, circuit);
}

#[allow(unreachable_patterns)]
fn test_shared_mem_helper<T: MPISharedMemory + ExpSerde + std::fmt::Debug>(
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

fn test_shared_mem_on_heap_helper<T: MPISharedMemory + ExpSerde + std::fmt::Debug + Default>(
    mpi_config: &MPIConfig,
    t: Option<T>,
) {
    let data_on_heap = Arc::new(Mutex::new(T::default()));

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

    *data_on_heap.lock().unwrap() = data;

    let mut shared_serialization = vec![];
    data_on_heap
        .lock()
        .unwrap()
        .serialize_into(&mut shared_serialization)
        .unwrap();

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

    match Arc::try_unwrap(data_on_heap) {
        Ok(mutex) => {
            let value = mutex.into_inner().unwrap(); // moves the value out
            value.discard_control_of_shared_mem();
        }
        Err(_) => {
            panic!("Failed to unwrap Arc, multiple references exist");
        }
    }

    mpi_config.free_shared_mem(&mut window);
}
