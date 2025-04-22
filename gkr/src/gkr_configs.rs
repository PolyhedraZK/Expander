use config_macros::declare_gkr_config;
use gf2::GF2x128;
use gkr_engine::{
    BN254Config, BabyBearExtConfig, FieldEngine, GF2ExtConfig, GKREngine, GKRScheme,
    GoldilocksConfig, GoldilocksExtConfig, M31Config, M31ExtConfig, MPIConfig,
};
use gkr_hashers::{MiMC5FiatShamirHasher, PoseidonFiatShamirHasher, SHA256hasher};
use goldilocks::Goldilocksx8;
use halo2curves::bn256::{Bn256, G1Affine};
use mersenne31::M31x16;
use poly_commit::{raw::RawExpanderGKR, HyperKZGPCS, HyraxPCS, OrionPCSForGKR};
use transcript::{BytesHashTranscript, FieldHashTranscript};

// ============== M31 ==============
declare_gkr_config!(
    pub M31ConfigSha2RawVanilla,
    FieldType::M31,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);
// ============== M31Ext3 ==============
declare_gkr_config!(
    pub M31ExtConfigPoseidonRawVanilla,
    FieldType::M31Ext3,
    FiatShamirHashType::Poseidon,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);
declare_gkr_config!(
    pub M31ExtConfigPoseidonRawSquare,
    FieldType::M31Ext3,
    FiatShamirHashType::Poseidon,
    PolynomialCommitmentType::Raw,
    GKRScheme::GkrSquare,
);
declare_gkr_config!(
    pub M31ExtConfigSha2OrionVanilla,
    FieldType::M31Ext3,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Orion,
    GKRScheme::Vanilla,
);
declare_gkr_config!(
    pub M31ExtConfigSha2OrionSquare,
    FieldType::M31Ext3,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Orion,
    GKRScheme::GkrSquare,
);
declare_gkr_config!(
    pub M31ExtConfigSha2RawVanilla,
    FieldType::M31Ext3,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);
declare_gkr_config!(
    pub M31ExtConfigSha2RawSquare,
    FieldType::M31Ext3,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::GkrSquare,
);

// ============== BN254 ==============
declare_gkr_config!(
    pub BN254ConfigMIMC5Raw,
    FieldType::BN254,
    FiatShamirHashType::MIMC5,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);
declare_gkr_config!(
    pub BN254ConfigSha2Raw,
    FieldType::BN254,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);
declare_gkr_config!(
    pub BN254ConfigSha2Hyrax,
    FieldType::BN254,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Hyrax,
    GKRScheme::Vanilla,
);
declare_gkr_config!(
    pub BN254ConfigMIMC5KZG,
    FieldType::BN254,
    FiatShamirHashType::MIMC5,
    PolynomialCommitmentType::KZG,
    GKRScheme::Vanilla,
);

// ============== GF2 ==============
declare_gkr_config!(
    pub GF2ExtConfigSha2Orion,
    FieldType::GF2Ext128,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Orion,
    GKRScheme::Vanilla,
);
declare_gkr_config!(
    pub GF2ExtConfigSha2Raw,
    FieldType::GF2Ext128,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);

// ============== Goldilocks ==============
declare_gkr_config!(
    pub GoldilocksConfigSha2Raw,
    FieldType::Goldilocks,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);

// ============== GoldilocksExt2 ==============
declare_gkr_config!(
    pub GoldilocksExtConfigSha2Raw,
    FieldType::GoldilocksExt2,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);

declare_gkr_config!(
    pub GoldilocksExtConfigSha2Orion,
    FieldType::GoldilocksExt2,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Orion,
    GKRScheme::Vanilla,
);

// ============== Babybear ==============
declare_gkr_config!(
    pub BabyBearExtConfigSha2Raw,
    FieldType::BabyBearExt3,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);
