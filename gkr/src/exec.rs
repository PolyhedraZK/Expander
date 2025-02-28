use std::str::FromStr;

use clap::Parser;
use config::{Config, FiatShamirHashType, GKRScheme, PolynomialCommitmentType};
use gkr::{executor::*, gkr_configs::*};
use gkr_field_config::FieldType;
use mpi_config::{root_println, MPIConfig};

#[tokio::main]
async fn main() {
    let expander_exec_args = ExpanderExecArgs::parse();

    let fs_hash_type = FiatShamirHashType::from_str(&expander_exec_args.fiat_shamir_hash).unwrap();
    let pcs_type =
        PolynomialCommitmentType::from_str(&expander_exec_args.poly_commitment_scheme).unwrap();

    let mpi_config = MPIConfig::new();

    root_println!(mpi_config, "Fiat-Shamir Hash Type: {:?}", &fs_hash_type);
    root_println!(
        mpi_config,
        "Polynomial Commitment Scheme Type: {:?}",
        &pcs_type
    );

    let field_type = detect_field_type_from_circuit_file(&expander_exec_args.circuit_file);
    root_println!(&mpi_config, "field type: {:?}", field_type);

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
        (FiatShamirHashType::SHA256, PolynomialCommitmentType::Hyrax, FieldType::BN254) => {
            run_command::<BN254ConfigSha2Hyrax>(
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
        (FiatShamirHashType::SHA256, PolynomialCommitmentType::Raw, FieldType::GF2) => {
            run_command::<GF2ExtConfigSha2Raw>(
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
