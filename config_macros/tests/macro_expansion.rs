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
        M31Sha256Config,
        FieldType::M31,
        FiatShamirHashType::SHA256,
        PolynomialCommitmentType::Raw,
        GKRScheme::Vanilla,
    );
    declare_gkr_config!(
        M31PoseidonRawConfig,
        FieldType::M31,
        FiatShamirHashType::Poseidon,
        PolynomialCommitmentType::Raw,
        GKRScheme::Vanilla,
    );
    declare_gkr_config!(
        M31PoseidonOrionConfig,
        FieldType::M31,
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
        GF2Keccak256Config,
        FieldType::GF2,
        FiatShamirHashType::Keccak256,
        PolynomialCommitmentType::Raw,
        GKRScheme::Vanilla,
    );
    declare_gkr_config!(
        GF2Keccak256OrionConfig,
        FieldType::GF2,
        FiatShamirHashType::Keccak256,
        PolynomialCommitmentType::Orion,
        GKRScheme::Vanilla,
    );
    declare_gkr_config!(
        GoldilocksSHA256Config,
        FieldType::Goldilocks,
        FiatShamirHashType::SHA256,
        PolynomialCommitmentType::Raw,
        GKRScheme::Vanilla,
    );

    print_type_name::<M31Sha256Config>();
    print_type_name::<M31PoseidonRawConfig>();
    print_type_name::<M31PoseidonOrionConfig>();
    print_type_name::<BN254MIMCConfig>();
    print_type_name::<BN254MIMCKZGConfig>();
    print_type_name::<GF2Keccak256Config>();
    print_type_name::<GF2Keccak256OrionConfig>();
    print_type_name::<GoldilocksSHA256Config>();
}
