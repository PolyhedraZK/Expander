use config::{Config, GKRScheme};
use mpi_config::MPIConfig;

use log::debug;

#[allow(unused_imports)] // The FieldType import is used in the macro expansion
use gkr_field_config::FieldType;

use gkr::{executor::*, gkr_configs::*};

#[tokio::main]
async fn main() {
    // examples:
    // expander-exec prove <input:circuit_file> <input:witness_file> <output:proof>
    // expander-exec verify <input:circuit_file> <input:witness_file> <input:proof> <input:mpi_size>
    // expander-exec serve <input:circuit_file> <input:ip> <input:port>
    let mut mpi_config = MPIConfig::new();

    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 5 {
        println!(
            "Usage: expander-exec prove <input:circuit_file> <input:witness_file> <output:proof>"
        );
        println!(
            "Usage: expander-exec verify <input:circuit_file> <input:witness_file> <input:proof> <input:mpi_size>"
        );
        println!("Usage: expander-exec serve <input:circuit_file> <input:host> <input:port>");
        return;
    }
    let command = &args[1];
    if command != "prove" && command != "verify" && command != "serve" {
        println!("Invalid command.");
        return;
    }

    if command == "verify" && args.len() > 5 {
        assert!(mpi_config.world_size == 1); // verifier should not be run with mpiexec
        mpi_config.world_size = args[5].parse::<i32>().expect("Parsing mpi size fails");
    }

    let circuit_file = &args[2];
    let field_type = detect_field_type_from_circuit_file(circuit_file);
    debug!("field type: {:?}", field_type);
    match field_type {
        FieldType::M31 => {
            run_command::<M31ExtConfigPoseidonOrion>(
                command,
                circuit_file,
                Config::<M31ExtConfigPoseidonOrion>::new(GKRScheme::Vanilla, mpi_config.clone()),
                &args,
            )
            .await;
        }
        FieldType::BN254 => {
            run_command::<BN254ConfigMIMC5Raw>(
                command,
                circuit_file,
                Config::<BN254ConfigMIMC5Raw>::new(GKRScheme::Vanilla, mpi_config.clone()),
                &args,
            )
            .await;
        }
        FieldType::GF2 => {
            run_command::<GF2ExtConfigSha2Orion>(
                command,
                circuit_file,
                Config::<GF2ExtConfigSha2Orion>::new(GKRScheme::Vanilla, mpi_config.clone()),
                &args,
            )
            .await
        }
    }

    MPIConfig::finalize();
}
