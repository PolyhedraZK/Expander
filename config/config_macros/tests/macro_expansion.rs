use std::any::type_name;

#[allow(unused_imports)] // The FiatShamirHashType import is used in the macro expansion
use config::{FiatShamirHashType, PolynomialCommitmentType};
#[allow(unused_imports)] // The FieldType import is used in the macro expansion
use gkr_field_config::FieldType;

use config::GKRConfig;
use config_macros::declare_gkr_config;
use gkr_field_config::{BN254Config, GF2ExtConfig, GKRFieldConfig, M31ExtConfig};
use polynomial_commitment_scheme::raw::RawExpanderGKR;
use transcript::{
    BytesHashTranscript, FieldHashTranscript, Keccak256hasher, MIMCHasher, SHA256hasher,
};

fn print_type_name<Cfg: GKRConfig>() {
    println!("{}", type_name::<Cfg>());
}

#[test]
fn main() {
    declare_gkr_config!(
        M31Sha256Config,
        FieldType::M31,
        FiatShamirHashType::SHA256,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        BN254MIMCConfig,
        FieldType::BN254,
        FiatShamirHashType::MIMC5,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        GF2Keccak256Config,
        FieldType::GF2,
        FiatShamirHashType::Keccak256,
        PolynomialCommitmentType::Raw
    );

    print_type_name::<M31Sha256Config>();
    print_type_name::<BN254MIMCConfig>();
    print_type_name::<GF2Keccak256Config>();
}
