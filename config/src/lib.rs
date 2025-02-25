use arith::Field;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use poly_commit::PCSForExpanderGKR;
use std::{fmt::Debug, str::FromStr};
use thiserror::Error;
use transcript::Transcript;

#[derive(Debug, Error)]
pub enum ConfigEnumFromStrError {
    #[error("Unknown string `{0}` for config enum deserialize")]
    SerializationError(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum PolynomialCommitmentType {
    #[default]
    Raw,
    KZG,
    Hyrax,
    Orion,
    FRI,
}

impl FromStr for PolynomialCommitmentType {
    type Err = ConfigEnumFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Raw" => Ok(PolynomialCommitmentType::Raw),
            "KZG" => Ok(PolynomialCommitmentType::KZG),
            "Hyrax" => Ok(PolynomialCommitmentType::Hyrax),
            "Orion" => Ok(PolynomialCommitmentType::Orion),
            "FRI" => Ok(PolynomialCommitmentType::FRI),
            _ => Err(ConfigEnumFromStrError::SerializationError(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum FiatShamirHashType {
    #[default]
    SHA256,
    Keccak256,
    Poseidon,
    Animoe,
    MIMC5, // Note: use MIMC5 for bn254 ONLY
}

impl FromStr for FiatShamirHashType {
    type Err = ConfigEnumFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SHA256" => Ok(FiatShamirHashType::SHA256),
            "Keccak256" => Ok(FiatShamirHashType::Keccak256),
            "Poseidon" => Ok(FiatShamirHashType::Poseidon),
            "Animoe" => Ok(FiatShamirHashType::Animoe),
            "MIMC5" => Ok(FiatShamirHashType::MIMC5),
            _ => Err(ConfigEnumFromStrError::SerializationError(s.to_string())),
        }
    }
}

pub const SENTINEL_M31: [u8; 32] = [
    255, 255, 255, 127, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0,
];

pub const SENTINEL_BN254: [u8; 32] = [
    1, 0, 0, 240, 147, 245, 225, 67, 145, 112, 185, 121, 72, 232, 51, 40, 93, 88, 129, 129, 182,
    69, 80, 184, 41, 160, 49, 225, 114, 78, 100, 48,
];

pub const SENTINEL_GF2: [u8; 32] = [
    2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub trait GKRConfig: Default + Debug + Clone + Send + Sync + 'static {
    /// Field config for gkr
    type FieldConfig: GKRFieldConfig;

    /// Fiat Shamir hash type. This defines the transcript type to use
    const FIAT_SHAMIR_HASH: FiatShamirHashType;

    /// The transcript type
    type Transcript: Transcript<<Self::FieldConfig as GKRFieldConfig>::ChallengeField>;

    /// The Polynomial Commitment type
    const PCS_TYPE: PolynomialCommitmentType;

    /// The specific Polynomial Commitment type
    type PCS: PCSForExpanderGKR<Self::FieldConfig, Self::Transcript>;
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum GKRScheme {
    #[default]
    Vanilla,
    GkrSquare,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config<C: GKRConfig> {
    // Field size for the variables
    pub field_size: usize,
    // Targeted security level for the scheme
    pub security_bits: usize,
    #[cfg(feature = "grinding")]
    // Grinding bits to achieve the target security level
    pub grinding_bits: usize,
    // Polynomial commitment scheme
    pub polynomial_commitment_type: PolynomialCommitmentType,
    // Field configuration for GKR
    pub gkr_config: C,
    // Whether to use GKR^2
    pub gkr_scheme: GKRScheme,
    // mpi config
    pub mpi_config: MPIConfig,
}

impl<C: GKRConfig> Config<C> {
    pub fn new(gkr_scheme: GKRScheme, mpi_config: MPIConfig) -> Self {
        Config {
            field_size: <C::FieldConfig as GKRFieldConfig>::ChallengeField::FIELD_SIZE,
            security_bits: 100,
            #[cfg(feature = "grinding")]
            grinding_bits: 10,
            polynomial_commitment_type: PolynomialCommitmentType::Raw,
            gkr_config: C::default(),
            gkr_scheme,
            mpi_config,
        }
    }
}
