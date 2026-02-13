use config_macros::declare_gkr_config;
use gkr_engine::{BN254Config, FieldEngine, GKREngine, GKRScheme, MPIConfig};
use gkr_hashers::{MiMC5FiatShamirHasher, SHA256hasher};
use halo2curves::bn256::{Bn256, G1Affine};
use poly_commit::{raw::RawExpanderGKR, HyperBiKZGPCS, HyraxPCS};
use transcript::BytesHashTranscript;

#[cfg(feature = "mersenne31")]
use gkr_engine::{M31x16Config, M31x1Config};
#[cfg(feature = "mersenne31")]
use gkr_hashers::PoseidonFiatShamirHasher;
#[cfg(feature = "mersenne31")]
use mersenne31::M31x16;
#[cfg(any(feature = "mersenne31", feature = "gf2", feature = "goldilocks"))]
use poly_commit::OrionPCSForGKR;
#[cfg(all(feature = "gf2", feature = "gf2_128"))]
use gf2::GF2x128;
#[cfg(all(feature = "gf2", feature = "gf2_128"))]
use gkr_engine::GF2ExtConfig;
#[cfg(feature = "goldilocks")]
use gkr_engine::{Goldilocksx1Config, Goldilocksx8Config};
#[cfg(feature = "goldilocks")]
use goldilocks::Goldilocksx8;
#[cfg(feature = "babybear")]
use gkr_engine::BabyBearx16Config;

// ============== M31 ==============
#[cfg(feature = "mersenne31")]
declare_gkr_config!(
    pub M31x1ConfigSha2RawVanilla,
    FieldType::M31x1,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);
// ============== M31Ext3 ==============
#[cfg(feature = "mersenne31")]
declare_gkr_config!(
    pub M31x16ConfigPoseidonRawVanilla,
    FieldType::M31x16,
    FiatShamirHashType::Poseidon,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);
#[cfg(feature = "mersenne31")]
declare_gkr_config!(
    pub M31x16ConfigPoseidonRawSquare,
    FieldType::M31x16,
    FiatShamirHashType::Poseidon,
    PolynomialCommitmentType::Raw,
    GKRScheme::GkrSquare,
);
#[cfg(feature = "mersenne31")]
declare_gkr_config!(
    pub M31x16ConfigSha2OrionVanilla,
    FieldType::M31x16,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Orion,
    GKRScheme::Vanilla,
);
#[cfg(feature = "mersenne31")]
declare_gkr_config!(
    pub M31x16ConfigSha2OrionSquare,
    FieldType::M31x16,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Orion,
    GKRScheme::GkrSquare,
);
#[cfg(feature = "mersenne31")]
declare_gkr_config!(
    pub M31x16ConfigSha2RawVanilla,
    FieldType::M31x16,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);
#[cfg(feature = "mersenne31")]
declare_gkr_config!(
    pub M31x16ConfigSha2RawSquare,
    FieldType::M31x16,
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
    pub BN254ConfigSha2KZG,
    FieldType::BN254,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::KZG,
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
#[cfg(all(feature = "gf2", feature = "gf2_128"))]
declare_gkr_config!(
    pub GF2ExtConfigSha2Orion,
    FieldType::GF2Ext128,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Orion,
    GKRScheme::Vanilla,
);
#[cfg(all(feature = "gf2", feature = "gf2_128"))]
declare_gkr_config!(
    pub GF2ExtConfigSha2Raw,
    FieldType::GF2Ext128,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);

// ============== Goldilocks ==============
#[cfg(feature = "goldilocks")]
declare_gkr_config!(
    pub Goldilocksx1ConfigSha2Raw,
    FieldType::Goldilocksx1,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);

// ============== GoldilocksExt2 ==============
#[cfg(feature = "goldilocks")]
declare_gkr_config!(
    pub Goldilocksx8ConfigSha2Raw,
    FieldType::Goldilocksx8,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);

#[cfg(feature = "goldilocks")]
declare_gkr_config!(
    pub Goldilocksx8ConfigSha2Orion,
    FieldType::Goldilocksx8,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Orion,
    GKRScheme::Vanilla,
);

// ============== Babybear ==============
#[cfg(feature = "babybear")]
declare_gkr_config!(
    pub BabyBearx16ConfigSha2Raw,
    FieldType::BabyBearx16,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw,
    GKRScheme::Vanilla,
);
