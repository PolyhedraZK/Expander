use std::str::FromStr;

use clap::Parser;
use config::{Config, FiatShamirHashType, GKRScheme, PolynomialCommitmentType};
use mpi_config::MPIConfig;

#[allow(unused_imports)] // The FieldType import is used in the macro expansion
use gkr_field_config::FieldType;

use gkr::{executor::*, gkr_configs::*};

#[tokio::main]
async fn main() {
    let expander_exec_args = ExpanderExecArgs::try_parse().unwrap();

    // TODO(HS) better pretty print and error message handling
    println!("{:?}", expander_exec_args);

    let fs_hash_type = FiatShamirHashType::from_str(&expander_exec_args.fiat_shamir_hash).unwrap();
    let pcs_type =
        PolynomialCommitmentType::from_str(&expander_exec_args.poly_commitment_scheme).unwrap();

    println!("Fiat-Shamir Hash Type: {:?}", &fs_hash_type);
    println!("Polynomial Commitment Scheme Type: {:?}", &pcs_type);

    let mut mpi_config = MPIConfig::new();

    if let ExpanderExecSubCommand::Verify {
        witness_file: _,
        input_proof_file: _,
        mpi_size,
    } = &expander_exec_args.subcommands
    {
        assert_eq!(mpi_config.world_size, 1);
        mpi_config.world_size = *mpi_size as i32;
    }

    let field_type = detect_field_type_from_circuit_file(&expander_exec_args.circuit_file);
    println!("field type: {:?}", field_type);

    match (fs_hash_type.clone(), pcs_type.clone(), field_type.clone()) {
        (FiatShamirHashType::SHA256, PolynomialCommitmentType::Orion, FieldType::M31) => {
            run_command::<M31ExtConfigSha2Orion>(
                &expander_exec_args,
                Config::new(GKRScheme::Vanilla, mpi_config.clone()),
            )
            .await;
        }
        (FiatShamirHashType::Poseidon, PolynomialCommitmentType::Raw, FieldType::M31) => {
            run_command::<M31ExtConfigPoseidonRaw>(
                &expander_exec_args,
                Config::new(GKRScheme::Vanilla, mpi_config.clone()),
            )
            .await;
        }
        (FiatShamirHashType::MIMC5, PolynomialCommitmentType::Raw, FieldType::BN254) => {
            run_command::<BN254ConfigMIMC5Raw>(
                &expander_exec_args,
                Config::new(GKRScheme::Vanilla, mpi_config.clone()),
            )
            .await;
        }
        (FiatShamirHashType::SHA256, PolynomialCommitmentType::Raw, FieldType::BN254) => {
            run_command::<BN254ConfigSha2Raw>(
                &expander_exec_args,
                Config::new(GKRScheme::Vanilla, mpi_config.clone()),
            )
            .await;
        }
        (FiatShamirHashType::SHA256, PolynomialCommitmentType::Orion, FieldType::GF2) => {
            run_command::<GF2ExtConfigSha2Orion>(
                &expander_exec_args,
                Config::new(GKRScheme::Vanilla, mpi_config.clone()),
            )
            .await;
        }
        _ => panic!(
            "FS: {:?}, PCS: {:?}, Field: {:?} setting is not yet integrated in expander-exec",
            fs_hash_type, pcs_type, field_type
        ),
    }

    MPIConfig::finalize();
}
