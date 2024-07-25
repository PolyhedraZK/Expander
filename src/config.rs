use arith::{BinomialExtensionField, Field, FieldSerde, M31Ext3, SimdField, SimdM31Ext3, M31};
use halo2curves::bn256::Fr;

#[derive(Debug, Clone, PartialEq)]
pub enum PolynomialCommitmentType {
    Raw,
    KZG,
    Orion,
    FRI,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    M31,
    BabyBear,
    BN254,
}

pub const SENTINEL_M31: [u8; 32] = [
    255, 255, 255, 127, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0,
];

pub const SENTINEL_BN254: [u8; 32] = [
    1, 0, 0, 240, 147, 245, 225, 67, 145, 112, 185, 121, 72, 232, 51, 40, 93, 88, 129, 129, 182,
    69, 80, 184, 41, 160, 49, 225, 114, 78, 100, 48,
];

#[derive(Debug, Clone, PartialEq)]
pub enum FiatShamirHashType {
    SHA256,
    Keccak256,
    Poseidon,
    Animoe,
    MIMC7,
}

pub trait GKRConfig: Default + Clone + Send + 'static {
    /// Field type for the circuit, e.g., M31
    type CircuitField: Field + FieldSerde + Send;

    /// Field type for the challenge, e.g., M31Ext3
    type ChallengeField: BinomialExtensionField<BaseField = Self::CircuitField> + Send;

    /// Main field type for the scheme, e.g., SimdM31Ext3
    type Field: BinomialExtensionField + SimdField<Scalar = Self::ChallengeField> + Send;

    /// Field size for the main field
    const FIELD_SIZE: usize;

    /// Targeted security level for the scheme
    const SECURITY_BITS: usize;

    /// Grinding bits to achieve the target security level
    #[cfg(feature = "grinding")]
    const GRINDING_BITS: usize;

    /// Polynomial commitment scheme
    const POLYNOMIAL_COMMITMENT_TYPE: PolynomialCommitmentType;

    /// Enum type for Self::Field
    const FIELD_TYPE: FieldType;

    /// Configuration for FIAT-SHAMIR transformation
    const FS_HASH: FiatShamirHashType;

    /// If this is a GKR^2 scheme
    const GKR_SQUARE: bool;

    /// API to allow for multiplications between the challenge and the circuit field
    fn challenge_mul_circuit_field(
        a: &Self::ChallengeField,
        b: &Self::CircuitField,
    ) -> Self::ChallengeField;

    /// API to allow for multiplications between the main field and the circuit field
    fn field_mul_circuit_field(a: &Self::Field, b: &Self::CircuitField) -> Self::Field;

    /// API to allow for addition between the main field and the circuit field
    fn field_add_circuit_field(a: &Self::Field, b: &Self::CircuitField) -> Self::Field;
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct M31ExtConfig;

impl GKRConfig for M31ExtConfig {
    type CircuitField = M31;

    type ChallengeField = M31Ext3;

    type Field = SimdM31Ext3;

    const FIELD_SIZE: usize = 93;

    const SECURITY_BITS: usize = 100;

    #[cfg(feature = "grinding")]
    const GRINDING_BITS: usize = 10;

    const POLYNOMIAL_COMMITMENT_TYPE: PolynomialCommitmentType = PolynomialCommitmentType::Raw;

    const FIELD_TYPE: FieldType = FieldType::M31;

    const FS_HASH: FiatShamirHashType = FiatShamirHashType::SHA256;

    const GKR_SQUARE: bool = false;

    #[inline(always)]
    fn challenge_mul_circuit_field(
        a: &Self::ChallengeField,
        b: &Self::CircuitField,
    ) -> Self::ChallengeField {
        a.mul_by_base_field(b)
    }

    #[inline(always)]
    fn field_mul_circuit_field(a: &Self::Field, b: &Self::CircuitField) -> Self::Field {
        // directly multiply M31Ext3 with M31
        // skipping the conversion M31 -> M31Ext3
        *a * *b
    }

    #[inline(always)]
    fn field_add_circuit_field(a: &Self::Field, b: &Self::CircuitField) -> Self::Field {
        // directly add M31Ext3 with M31
        // skipping the conversion M31 -> M31Ext3
        *a + *b
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BN254Config;

impl GKRConfig for BN254Config {
    type CircuitField = Fr;

    type ChallengeField = Fr;

    type Field = Fr;

    const FIELD_SIZE: usize = 254;

    const SECURITY_BITS: usize = 100;

    #[cfg(feature = "grinding")]
    const GRINDING_BITS: usize = 0;

    const POLYNOMIAL_COMMITMENT_TYPE: PolynomialCommitmentType = PolynomialCommitmentType::Raw;

    const FIELD_TYPE: FieldType = FieldType::BN254;

    const FS_HASH: FiatShamirHashType = FiatShamirHashType::SHA256;

    const GKR_SQUARE: bool = false;

    #[inline(always)]
    fn challenge_mul_circuit_field(
        a: &Self::ChallengeField,
        b: &Self::CircuitField,
    ) -> Self::ChallengeField {
        a * b
    }

    #[inline(always)]
    fn field_mul_circuit_field(a: &Self::Field, b: &Self::CircuitField) -> Self::Field {
        a * b
    }

    #[inline(always)]
    fn field_add_circuit_field(a: &Self::Field, b: &Self::CircuitField) -> Self::Field {
        *a + *b
    }
}
