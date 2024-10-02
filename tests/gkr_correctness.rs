use expander_rs::{
    root_println, utils::*, BabyBearExt4ConfigKeccak, BabyBearExt4ConfigSha2, FieldType, MPIConfig,
};
use expander_rs::{
    BN254ConfigKeccak, BN254ConfigSha2, Circuit, Config, GF2ExtConfigKeccak, GF2ExtConfigSha2,
    GKRConfig, GKRScheme, M31ExtConfigKeccak, M31ExtConfigSha2, Prover, Verifier,
};
use std::panic;
use std::panic::AssertUnwindSafe;
use std::time::Instant;

use rand::Rng;
use sha2::Digest;

#[test]
fn test_gkr_correctness() {
    let mpi_config = MPIConfig::new();

    test_gkr_correctness_helper::<GF2ExtConfigSha2>(&Config::<GF2ExtConfigSha2>::new(
        GKRScheme::Vanilla,
        mpi_config.clone(),
    ));
    test_gkr_correctness_helper::<GF2ExtConfigKeccak>(&Config::<GF2ExtConfigKeccak>::new(
        GKRScheme::Vanilla,
        mpi_config.clone(),
    ));
    test_gkr_correctness_helper::<M31ExtConfigSha2>(&Config::<M31ExtConfigSha2>::new(
        GKRScheme::Vanilla,
        mpi_config.clone(),
    ));
    test_gkr_correctness_helper::<M31ExtConfigKeccak>(&Config::<M31ExtConfigKeccak>::new(
        GKRScheme::Vanilla,
        mpi_config.clone(),
    ));
    test_gkr_correctness_helper::<BN254ConfigSha2>(&Config::<BN254ConfigSha2>::new(
        GKRScheme::Vanilla,
        mpi_config.clone(),
    ));
    test_gkr_correctness_helper::<BN254ConfigKeccak>(&Config::<BN254ConfigKeccak>::new(
        GKRScheme::Vanilla,
        mpi_config.clone(),
    ));
    test_gkr_correctness_helper::<BabyBearExt4ConfigSha2>(&Config::<BabyBearExt4ConfigSha2>::new(
        GKRScheme::Vanilla,
        mpi_config.clone(),
    ));
    test_gkr_correctness_helper::<BabyBearExt4ConfigKeccak>(
        &Config::<BabyBearExt4ConfigKeccak>::new(GKRScheme::Vanilla, mpi_config.clone()),
    );

    MPIConfig::finalize();
}

#[allow(unreachable_patterns)]
fn test_gkr_correctness_helper<C: GKRConfig>(config: &Config<C>) {
    root_println!(config.mpi_config, "============== start ===============");
    root_println!(config.mpi_config, "Field Type: {:?}", C::FIELD_TYPE);
    let circuit_copy_size: usize = match C::FIELD_TYPE {
        FieldType::BabyBear => 2,
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
        FieldType::GF2 => KECCAK_GF2_CIRCUIT,
        _ => KECCAK_M31_CIRCUIT, // Use this for BabyBear, M31 and BN254-Fr
    };

    let mut circuit = Circuit::<C>::load_circuit(circuit_path);
    root_println!(config.mpi_config, "Circuit loaded.");

    circuit.set_random_input_for_test();

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
    // first and last 16 proof u8
    root_println!(config.mpi_config, "Proof bytes: ");
    proof.bytes.iter().take(16).for_each(|b| print!("{} ", b));
    print!("... ");
    proof
        .bytes
        .iter()
        .rev()
        .take(16)
        .rev()
        .for_each(|b| print!("{} ", b));
    root_println!(config.mpi_config,);

    root_println!(config.mpi_config, "Proof hash: ");
    sha2::Sha256::digest(&proof.bytes)
        .iter()
        .for_each(|b| print!("{} ", b));
    root_println!(config.mpi_config,);

    // Verify
    if config.mpi_config.is_root() {
        let verifier = Verifier::new(config);
        println!("Verifier created.");
        let verification_start = Instant::now();
        assert!(verifier.verify(&mut circuit, &claimed_v, &proof),);
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
            verifier.verify(&mut circuit, &claimed_v, &bad_proof)
        }));

        let final_result = result.unwrap_or_default();

        assert!(!final_result,);
        println!("Bad proof rejected.");
        println!("============== end ===============");
    }
}
