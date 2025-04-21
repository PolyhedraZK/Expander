use std::str::FromStr;

use clap::Parser;
use gkr::{executor::*, gkr_configs::*};
use gkr_engine::{
    root_println, FiatShamirHashType, FieldType, MPIConfig, MPIEngine, PolynomialCommitmentType,
};

#[tokio::main]
async fn main() {
    let expander_exec_args = ExpanderExecArgs::parse();

    let fs_hash_type = FiatShamirHashType::from_str(&expander_exec_args.fiat_shamir_hash).unwrap();
    let pcs_type =
        PolynomialCommitmentType::from_str(&expander_exec_args.poly_commitment_scheme).unwrap();

    let mpi_config = MPIConfig::prover_new();

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
        (FiatShamirHashType::SHA256, PolynomialCommitmentType::Orion, FieldType::M31Ext3) => {
            run_command::<M31ExtConfigSha2OrionVanilla>(&expander_exec_args, &mpi_config).await;
        }
        (FiatShamirHashType::Poseidon, PolynomialCommitmentType::Raw, FieldType::M31Ext3) => {
            run_command::<M31ExtConfigPoseidonRawVanilla>(&expander_exec_args, &mpi_config).await;
        }
        (FiatShamirHashType::MIMC5, PolynomialCommitmentType::Raw, FieldType::BN254) => {
            run_command::<BN254ConfigMIMC5Raw>(&expander_exec_args, &mpi_config).await;
        }
        (FiatShamirHashType::SHA256, PolynomialCommitmentType::Raw, FieldType::BN254) => {
            run_command::<BN254ConfigSha2Raw>(&expander_exec_args, &mpi_config).await;
        }
        (FiatShamirHashType::SHA256, PolynomialCommitmentType::Hyrax, FieldType::BN254) => {
            run_command::<BN254ConfigSha2Hyrax>(&expander_exec_args, &mpi_config).await;
        }
        (FiatShamirHashType::MIMC5, PolynomialCommitmentType::KZG, FieldType::BN254) => {
            run_command::<BN254ConfigMIMC5KZG>(&expander_exec_args, &mpi_config).await;
        }
        (FiatShamirHashType::SHA256, PolynomialCommitmentType::Orion, FieldType::GF2Ext128) => {
            run_command::<GF2ExtConfigSha2Orion>(&expander_exec_args, &mpi_config).await;
        }
        (FiatShamirHashType::SHA256, PolynomialCommitmentType::Raw, FieldType::GF2Ext128) => {
            run_command::<GF2ExtConfigSha2Raw>(&expander_exec_args, &mpi_config).await;
        }
        (FiatShamirHashType::SHA256, PolynomialCommitmentType::Orion, FieldType::Goldilocks) => {
            run_command::<GoldilocksExtConfigSha2Orion>(&expander_exec_args, &mpi_config).await;
        }
        _ => panic!(
            "FS: {:?}, PCS: {:?}, Field: {:?} setting is not yet integrated in expander-exec",
            fs_hash_type, pcs_type, field_type
        ),
    }

    MPIConfig::finalize();
}
