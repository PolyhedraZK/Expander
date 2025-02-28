use std::io::Write;
use std::panic::AssertUnwindSafe;
use std::time::Instant;
use std::{fs, panic};

use arith::{Field, FieldSerde};
use circuit::Circuit;
use config::{Config, FiatShamirHashType, GKRConfig, GKRScheme, PolynomialCommitmentType};
use config_macros::declare_gkr_config;
use field_hashers::{MiMC5FiatShamirHasher, PoseidonFiatShamirHasher};
use gf2::GF2x128;
use gkr_field_config::{BN254Config, FieldType, GF2ExtConfig, GKRFieldConfig, M31ExtConfig};
use halo2curves::bn256::G1Affine;
use mersenne31::M31x16;
use mpi_config::{root_println, MPIConfig};
use poly_commit::{expander_pcs_init_testing_only, HyraxPCS, OrionPCSForGKR, RawExpanderGKR};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha12Rng;
use sha2::Digest;
use transcript::{BytesHashTranscript, FieldHashTranscript, Keccak256hasher, SHA256hasher};

use crate::{utils::*, Prover, Verifier};

const PCS_TESTING_SEED_U64: u64 = 114514;

#[test]
fn test_gkr_correctness() {
    let mpi_config = MPIConfig::new();
    declare_gkr_config!(
        C0,
        FieldType::GF2,
        FiatShamirHashType::SHA256,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        C1,
        FieldType::M31,
        FiatShamirHashType::SHA256,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        C2,
        FieldType::BN254,
        FiatShamirHashType::SHA256,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        C3,
        FieldType::GF2,
        FiatShamirHashType::Keccak256,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        C4,
        FieldType::M31,
        FiatShamirHashType::Keccak256,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        C5,
        FieldType::BN254,
        FiatShamirHashType::Keccak256,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        C6,
        FieldType::BN254,
        FiatShamirHashType::MIMC5,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        C7,
        FieldType::GF2,
        FiatShamirHashType::Keccak256,
        PolynomialCommitmentType::Orion,
    );
    declare_gkr_config!(
        C8,
        FieldType::M31,
        FiatShamirHashType::Poseidon,
        PolynomialCommitmentType::Raw,
    );
    declare_gkr_config!(
        C9,
        FieldType::M31,
        FiatShamirHashType::Poseidon,
        PolynomialCommitmentType::Orion,
    );
    declare_gkr_config!(
        C10,
        FieldType::BN254,
        FiatShamirHashType::Keccak256,
        PolynomialCommitmentType::Hyrax
    );

    test_gkr_correctness_helper(
        &Config::<C0>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper(
        &Config::<C1>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper(
        &Config::<C2>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper(
        &Config::<C3>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper(
        &Config::<C4>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper(
        &Config::<C5>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper(
        &Config::<C6>::new(GKRScheme::Vanilla, mpi_config.clone()),
        Some("../data/gkr_proof.txt"),
    );
    test_gkr_correctness_helper(
        &Config::<C7>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper(
        &Config::<C8>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper(
        &Config::<C9>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper(
        &Config::<C10>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );

    MPIConfig::finalize();
}

#[allow(unreachable_patterns)]
fn test_gkr_correctness_helper<Cfg: GKRConfig>(config: &Config<Cfg>, write_proof_to: Option<&str>) {
    root_println!(config.mpi_config, "============== start ===============");
    root_println!(
        config.mpi_config,
        "Field Type: {:?}",
        <Cfg::FieldConfig as GKRFieldConfig>::FIELD_TYPE
    );
    let circuit_copy_size: usize = match <Cfg::FieldConfig as GKRFieldConfig>::FIELD_TYPE {
        FieldType::GF2 => 1,
        FieldType::M31 => 2,
        FieldType::BN254 => 2,
        _ => unreachable!(),
    };
    root_println!(
        config.mpi_config,
        "Proving {} keccak instances at once.",
        circuit_copy_size * <Cfg::FieldConfig as GKRFieldConfig>::get_field_pack_size()
    );
    root_println!(config.mpi_config, "Config created.");

    let circuit_path = match <Cfg::FieldConfig as GKRFieldConfig>::FIELD_TYPE {
        FieldType::GF2 => "../".to_owned() + KECCAK_GF2_CIRCUIT,
        FieldType::M31 => "../".to_owned() + KECCAK_M31_CIRCUIT,
        FieldType::BN254 => "../".to_owned() + KECCAK_BN254_CIRCUIT,
        _ => unreachable!(),
    };
    let mut circuit = Circuit::<Cfg::FieldConfig>::load_circuit::<Cfg>(&circuit_path);
    root_println!(config.mpi_config, "Circuit loaded.");

    let witness_path = match <Cfg::FieldConfig as GKRFieldConfig>::FIELD_TYPE {
        FieldType::GF2 => "../".to_owned() + KECCAK_GF2_WITNESS,
        FieldType::M31 => "../".to_owned() + KECCAK_M31_WITNESS,
        FieldType::BN254 => "../".to_owned() + KECCAK_BN254_WITNESS,
        _ => unreachable!(),
    };
    circuit.load_witness_allow_padding_testing_only(&witness_path, &config.mpi_config);
    root_println!(config.mpi_config, "Witness loaded.");

    circuit.evaluate();
    let output = &circuit.layers.last().unwrap().output_vals;
    assert!(output[..circuit.expected_num_output_zeros]
        .iter()
        .all(|f| f.is_zero()));

    let mut prover = Prover::new(config);
    prover.prepare_mem(&circuit);

    let mut rng = ChaCha12Rng::seed_from_u64(PCS_TESTING_SEED_U64);
    let (pcs_params, pcs_proving_key, pcs_verification_key, mut pcs_scratch) =
        expander_pcs_init_testing_only::<Cfg::FieldConfig, Cfg::Transcript, Cfg::PCS>(
            circuit.log_input_size(),
            &config.mpi_config,
            &mut rng,
        );

    let proving_start = Instant::now();
    let (claimed_v, proof) = prover.prove(
        &mut circuit,
        &pcs_params,
        &pcs_proving_key,
        &mut pcs_scratch,
    );
    root_println!(
        config.mpi_config,
        "Proving time: {} μs",
        proving_start.elapsed().as_micros()
    );

    root_println!(
        config.mpi_config,
        "Proof generated. Size: {} bytes",
        proof.bytes.len()
    );
    root_println!(config.mpi_config,);

    root_println!(config.mpi_config, "Proof hash: ");
    sha2::Sha256::digest(&proof.bytes)
        .iter()
        .for_each(|b| print!("{} ", b));
    root_println!(config.mpi_config,);

    let mut public_input_gathered = if config.mpi_config.is_root() {
        vec![
            <Cfg::FieldConfig as GKRFieldConfig>::SimdCircuitField::ZERO;
            circuit.public_input.len() * config.mpi_config.world_size()
        ]
    } else {
        vec![]
    };
    config
        .mpi_config
        .gather_vec(&circuit.public_input, &mut public_input_gathered);

    // Verify
    if config.mpi_config.is_root() {
        if let Some(str) = write_proof_to {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(str)
                .unwrap();

            let mut buf = vec![];
            proof.serialize_into(&mut buf).unwrap();
            file.write_all(&buf).unwrap();
        }
        let verifier = Verifier::new(config);
        println!("Verifier created.");
        let verification_start = Instant::now();
        assert!(verifier.verify(
            &mut circuit,
            &public_input_gathered,
            &claimed_v,
            &pcs_params,
            &pcs_verification_key,
            &proof
        ));
        println!(
            "Verification time: {} μs",
            verification_start.elapsed().as_micros()
        );
        println!("Correct proof verified.");
        let mut bad_proof = proof.clone();
        let rng = &mut rand::thread_rng();
        let random_idx = rng.gen_range(0..bad_proof.bytes.len());
        let random_change = rng.gen_range(1..256) as u8;
        bad_proof.bytes[random_idx] ^= random_change;

        // Catch the panic and treat it as returning `false`
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            verifier.verify(
                &mut circuit,
                &public_input_gathered,
                &claimed_v,
                &pcs_params,
                &pcs_verification_key,
                &bad_proof,
            )
        }));

        let final_result = result.unwrap_or_default();

        assert!(!final_result,);
        println!("Bad proof rejected.");
        println!("============== end ===============");
    }
}
