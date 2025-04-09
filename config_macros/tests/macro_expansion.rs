use std::any::type_name;

use config_macros::declare_gkr_config;
use gf2::GF2x128;
use gkr_engine::{
    BN254Config, FieldEngine, GF2ExtConfig, GKREngine, GKRScheme, GoldilocksExtConfig,
    M31ExtConfig, MPIConfig,
};
use gkr_hashers::{Keccak256hasher, MiMC5FiatShamirHasher, PoseidonFiatShamirHasher, SHA256hasher};
use halo2curves::bn256::Bn256;
use mersenne31::M31x16;
use poly_commit::{HyperKZGPCS, OrionPCSForGKR, RawExpanderGKR};
use transcript::{BytesHashTranscript, FieldHashTranscript};

fn print_type_name<Cfg: GKREngine>() {
    println!("{}: {:?}", type_name::<Cfg>(), Cfg::SCHEME);
}

#[test]
fn main() {
    declare_gkr_config!(
        M31ExtSha256Config,
        FieldType::M31Ext3,
        FiatShamirHashType::SHA256,
        PolynomialCommitmentType::Raw,
        GKRScheme::Vanilla,
    );
    declare_gkr_config!(
        M31ExtPoseidonRawConfig,
        FieldType::M31Ext3,
        FiatShamirHashType::Poseidon,
        PolynomialCommitmentType::Raw,
        GKRScheme::Vanilla,
    );
    declare_gkr_config!(
        M31ExtPoseidonOrionConfig,
        FieldType::M31Ext3,
        FiatShamirHashType::Poseidon,
        PolynomialCommitmentType::Orion,
        GKRScheme::Vanilla,
    );
    declare_gkr_config!(
        BN254MIMCConfig,
        FieldType::BN254,
        FiatShamirHashType::MIMC5,
        PolynomialCommitmentType::Raw,
        GKRScheme::Vanilla,
    );
    declare_gkr_config!(
        BN254MIMCKZGConfig,
        FieldType::BN254,
        FiatShamirHashType::MIMC5,
        PolynomialCommitmentType::KZG,
        GKRScheme::Vanilla,
    );
    declare_gkr_config!(
        GF2ExtKeccak256Config,
        FieldType::GF2Ext128,
        FiatShamirHashType::Keccak256,
        PolynomialCommitmentType::Raw,
        GKRScheme::Vanilla,
    );
    declare_gkr_config!(
        GF2ExtKeccak256OrionConfig,
        FieldType::GF2Ext128,
        FiatShamirHashType::Keccak256,
        PolynomialCommitmentType::Orion,
        GKRScheme::Vanilla,
    );
    declare_gkr_config!(
        GoldilocksExtSHA256Config,
        FieldType::GoldilocksExt2,
        FiatShamirHashType::SHA256,
        PolynomialCommitmentType::Raw,
        GKRScheme::Vanilla,
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
