use std::any::type_name;

#[allow(unused_imports)] // The FiatShamirHashType import is used in the macro expansion
use config::{FiatShamirHashType, PolynomialCommitmentType};
#[allow(unused_imports)] // The FieldType import is used in the macro expansion
use gkr_field_config::FieldType;

use config::GKRConfig;
use config_macros::declare_gkr_config;
use field_hashers::{MiMC5FiatShamirHasher, PoseidonFiatShamirHasher};
use gf2::GF2x128;
use gkr_field_config::{BN254Config, GF2ExtConfig, GKRFieldConfig, M31ExtConfig};
use mersenne31::M31x16;
use poly_commit::{OrionPCSForGKR, RawExpanderGKR};
use transcript::{BytesHashTranscript, FieldHashTranscript, Keccak256hasher, SHA256hasher};

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
        M31PoseidonRawConfig,
        FieldType::M31,
        FiatShamirHashType::Poseidon,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        M31PoseidonOrionConfig,
        FieldType::M31,
        FiatShamirHashType::Poseidon,
        PolynomialCommitmentType::Orion
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
    declare_gkr_config!(
        GF2Keccak256OrionConfig,
        FieldType::GF2,
        FiatShamirHashType::Keccak256,
        PolynomialCommitmentType::Orion
    );

    print_type_name::<M31Sha256Config>();
    print_type_name::<M31PoseidonRawConfig>();
    print_type_name::<M31PoseidonOrionConfig>();
    print_type_name::<BN254MIMCConfig>();
    print_type_name::<GF2Keccak256Config>();
    print_type_name::<GF2Keccak256OrionConfig>();
}
