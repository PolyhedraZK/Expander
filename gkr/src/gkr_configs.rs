use config::{FiatShamirHashType, GKRConfig, PolynomialCommitmentType};
use config_macros::declare_gkr_config;
use field_hashers::{MiMC5FiatShamirHasher, PoseidonFiatShamirHasher};
use gf2::GF2x128;
use gkr_field_config::{BN254Config, GF2ExtConfig, GKRFieldConfig, M31ExtConfig};
use halo2curves::bn256::G1Affine;
use mersenne31::M31x16;
use poly_commit::{raw::RawExpanderGKR, HyraxPCS, OrionPCSForGKR};
use transcript::{BytesHashTranscript, FieldHashTranscript, SHA256hasher};

// ============== M31 ==============
declare_gkr_config!(
    pub M31ExtConfigPoseidonRaw,
    FieldType::M31,
    FiatShamirHashType::Poseidon,
    PolynomialCommitmentType::Raw
);
declare_gkr_config!(
    pub M31ExtConfigSha2Orion,
    FieldType::M31,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Orion
);
declare_gkr_config!(
    pub M31ExtConfigSha2Raw,
    FieldType::M31,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw
);

// ============== BN254 ==============
declare_gkr_config!(
    pub BN254ConfigMIMC5Raw,
    FieldType::BN254,
    FiatShamirHashType::MIMC5,
    PolynomialCommitmentType::Raw
);
declare_gkr_config!(
    pub BN254ConfigSha2Raw,
    FieldType::BN254,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw
);
declare_gkr_config!(
    pub BN254ConfigSha2Hyrax,
    FieldType::BN254,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Hyrax
);

// ============== GF2 ==============
declare_gkr_config!(
    pub GF2ExtConfigSha2Orion,
    FieldType::GF2,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Orion
);
declare_gkr_config!(
    pub GF2ExtConfigSha2Raw,
    FieldType::GF2,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw
);
