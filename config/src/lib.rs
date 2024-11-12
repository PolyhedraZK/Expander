use arith::Field;
use std::fmt::Debug;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use transcript::Transcript;


#[derive(Debug, Clone, PartialEq, Default)]
pub enum PolynomialCommitmentType {
    #[default]
    Raw,
    KZG,
    Orion,
    FRI,
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
    type FieldConfig: GKRFieldConfig;

    type Transcript: Transcript<<Self::FieldConfig as GKRFieldConfig>::ChallengeField>;
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
