use std::io::Write;
use std::panic::AssertUnwindSafe;
use std::time::Instant;
use std::{fs, panic};

use arith::{Field, FieldSerde};
use circuit::Circuit;
use config::{
    root_println, BN254ConfigKeccak, BN254ConfigMIMC5, BN254ConfigSha2, Config, FieldType,
    GF2ExtConfigKeccak, GF2ExtConfigSha2, GKRConfig, GKRScheme, M31ExtConfigKeccak,
    M31ExtConfigSha2, MPIConfig,
};
use rand::Rng;
use sha2::Digest;

use crate::{utils::*, Prover, Verifier};

#[test]
fn test_gkr_correctness() {
    let mpi_config = MPIConfig::new();

    test_gkr_correctness_helper::<GF2ExtConfigSha2>(
        &Config::<GF2ExtConfigSha2>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper::<GF2ExtConfigKeccak>(
        &Config::<GF2ExtConfigKeccak>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper::<M31ExtConfigSha2>(
        &Config::<M31ExtConfigSha2>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper::<M31ExtConfigKeccak>(
        &Config::<M31ExtConfigKeccak>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper::<BN254ConfigSha2>(
        &Config::<BN254ConfigSha2>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper::<BN254ConfigKeccak>(
        &Config::<BN254ConfigKeccak>::new(GKRScheme::Vanilla, mpi_config.clone()),
        None,
    );
    test_gkr_correctness_helper::<BN254ConfigMIMC5>(
        &Config::<BN254ConfigMIMC5>::new(GKRScheme::Vanilla, mpi_config.clone()),
        Some("../data/gkr_proof.txt"),
    );

    MPIConfig::finalize();
}

#[allow(unreachable_patterns)]
fn test_gkr_correctness_helper<C: GKRConfig>(config: &Config<C>, write_proof_to: Option<&str>) {
    root_println!(config.mpi_config, "============== start ===============");
    root_println!(config.mpi_config, "Field Type: {:?}", C::FIELD_TYPE);
    let circuit_copy_size: usize = match C::FIELD_TYPE {
        FieldType::GF2 => 1,
        FieldType::M31 => 2,
        FieldType::BN254 => 2,
        _ => unreachable!(),
    };
    root_println!(
        config.mpi_config,
        "Proving {} keccak instances at once.",
        circuit_copy_size * C::get_field_pack_size()
    );
    root_println!(config.mpi_config, "Config created.");

    let circuit_path = match C::FIELD_TYPE {
        FieldType::GF2 => "../".to_owned() + KECCAK_GF2_CIRCUIT,
        FieldType::M31 => "../".to_owned() + KECCAK_M31_CIRCUIT,
        FieldType::BN254 => "../".to_owned() + KECCAK_BN254_CIRCUIT,
        _ => unreachable!(),
    };
    let mut circuit = Circuit::<C>::load_circuit(&circuit_path);
    root_println!(config.mpi_config, "Circuit loaded.");

    let witness_path = match C::FIELD_TYPE {
        FieldType::GF2 => "../".to_owned() + KECCAK_GF2_WITNESS,
        FieldType::M31 => "../".to_owned() + KECCAK_M31_WITNESS,
        FieldType::BN254 => "../".to_owned() + KECCAK_BN254_WITNESS,
        _ => unreachable!(),
    };
    circuit.load_witness_file(&witness_path);
    root_println!(config.mpi_config, "Witness loaded.");

    circuit.evaluate();
    let output = &circuit.layers.last().unwrap().output_vals;
    assert!(output[..circuit.expected_num_output_zeros]
        .iter()
        .all(|f| f.is_zero()));

    let mut prover = Prover::new(config);
    prover.prepare_mem(&circuit);

    let proving_start = Instant::now();
    let (claimed_v, proof) = prover.prove(&mut circuit);
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
        vec![C::SimdCircuitField::ZERO; circuit.public_input.len() * config.mpi_config.world_size()]
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
        assert!(verifier.verify(&mut circuit, &public_input_gathered, &claimed_v, &proof));
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
            verifier.verify(&mut circuit, &public_input_gathered, &claimed_v, &bad_proof)
        }));

        let final_result = result.unwrap_or_default();

        assert!(!final_result,);
        println!("Bad proof rejected.");
        println!("============== end ===============");
    }
}
