use std::any::type_name;

#[allow(unused_imports)] // The FiatShamirHashType import is used in the macro expansion
use config::{FiatShamirHashType, PolynomialCommitmentType};
#[allow(unused_imports)] // The FieldType import is used in the macro expansion
use gkr_field_config::FieldType;

use config::GKRConfig;
use config_macros::declare_gkr_config;
use field_hashers::{MiMC5FiatShamirHasher, PoseidonFiatShamirHasher};
use gf2::GF2x128;
use gkr_field_config::{
    BN254Config, GF2ExtConfig, GKRFieldConfig, GoldilocksExtConfig, M31ExtConfig,
};
use halo2curves::bn256::Bn256;
use mersenne31::M31x16;
use poly_commit::{HyperKZGPCS, OrionPCSForGKR, RawExpanderGKR};
use transcript::{BytesHashTranscript, FieldHashTranscript, Keccak256hasher, SHA256hasher};

fn print_type_name<Cfg: GKRConfig>() {
    println!("{}", type_name::<Cfg>());
}

#[test]
fn main() {
    declare_gkr_config!(
        M31ExtSha256Config,
        FieldType::M31Ext3,
        FiatShamirHashType::SHA256,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        M31ExtPoseidonRawConfig,
        FieldType::M31Ext3,
        FiatShamirHashType::Poseidon,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        M31ExtPoseidonOrionConfig,
        FieldType::M31Ext3,
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
        BN254MIMCKZGConfig,
        FieldType::BN254,
        FiatShamirHashType::MIMC5,
        PolynomialCommitmentType::KZG
    );
    declare_gkr_config!(
        GF2ExtKeccak256Config,
        FieldType::GF2Ext128,
        FiatShamirHashType::Keccak256,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        GF2ExtKeccak256OrionConfig,
        FieldType::GF2Ext128,
        FiatShamirHashType::Keccak256,
        PolynomialCommitmentType::Orion
    );
    declare_gkr_config!(
        GoldilocksExtSHA256Config,
        FieldType::GoldilocksExt2,
        FiatShamirHashType::SHA256,
        PolynomialCommitmentType::Raw
    );

    print_type_name::<M31ExtSha256Config>();
    print_type_name::<M31ExtPoseidonRawConfig>();
    print_type_name::<M31ExtPoseidonOrionConfig>();
    print_type_name::<BN254MIMCConfig>();
    print_type_name::<BN254MIMCKZGConfig>();
    print_type_name::<GF2ExtKeccak256Config>();
    print_type_name::<GF2ExtKeccak256OrionConfig>();
    print_type_name::<GoldilocksExtSHA256Config>();
}
