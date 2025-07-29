use std::str::FromStr;

use bin::executor::*;
use clap::Parser;
use gkr::gkr_configs::*;
use gkr_engine::{
    FiatShamirHashType, FieldType, MPIConfig, MPIEngine, PolynomialCommitmentType, root_println,
};

#[tokio::main]
async fn main() {
    let expander_exec_args = ExpanderExecArgs::parse();

    let fs_hash_type = FiatShamirHashType::from_str(&expander_exec_args.fiat_shamir_hash).unwrap();
    let pcs_type =
        PolynomialCommitmentType::from_str(&expander_exec_args.poly_commitment_scheme).unwrap();

    let universe = MPIConfig::init().unwrap();
    let world = universe.world();
    let mpi_config = MPIConfig::prover_new(Some(&universe), Some(&world));
    root_println!(mpi_config, "Fiat-Shamir Hash Type: {:?}", &fs_hash_type);
    root_println!(
        mpi_config,
        "Polynomial Commitment Scheme Type: {:?}",
        &pcs_type
    );

    // Get circuit_file based on subcommand
    let circuit_file = match &expander_exec_args.subcommands {
        ExpanderExecSubCommand::Prove { circuit_file, .. } => circuit_file,
        ExpanderExecSubCommand::Verify { circuit_file, .. } => circuit_file,
        ExpanderExecSubCommand::Serve { circuit_file, .. } => circuit_file,
    };

    let field_type = detect_field_type_from_circuit_file(circuit_file);
    root_println!(&mpi_config, "field type: {:?}", field_type);

    match (fs_hash_type.clone(), pcs_type.clone(), field_type.clone()) {
        (FiatShamirHashType::Poseidon, PolynomialCommitmentType::Raw, FieldType::M31x16) => {
            run_command::<M31x16ConfigPoseidonRawVanilla>(&expander_exec_args, &mpi_config).await;
        }
        (FiatShamirHashType::MIMC5, PolynomialCommitmentType::Raw, FieldType::BN254) => {
            run_command::<BN254ConfigMIMC5Raw>(&expander_exec_args, &mpi_config).await;
        }
        (FiatShamirHashType::SHA256, PolynomialCommitmentType::Raw, FieldType::BN254) => {
            run_command::<BN254ConfigSha2Raw>(&expander_exec_args, &mpi_config).await;
        }
        (FiatShamirHashType::MIMC5, PolynomialCommitmentType::KZG, FieldType::BN254) => {
            run_command::<BN254ConfigMIMC5KZG>(&expander_exec_args, &mpi_config).await;
        }
        (FiatShamirHashType::SHA256, PolynomialCommitmentType::Raw, FieldType::GF2Ext128) => {
            run_command::<GF2ExtConfigSha2Raw>(&expander_exec_args, &mpi_config).await;
        }
        _ => panic!(
            "FS: {fs_hash_type:?}, PCS: {pcs_type:?}, Field: {field_type:?} setting is not yet integrated in expander-exec"
        ),
    }
}
