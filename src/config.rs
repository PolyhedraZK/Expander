use arith::{BinomialExtensionField, Field, FieldSerde, M31Ext3, SimdField, SimdM31Ext3, M31};

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

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub field_size: usize,
    pub security_bits: usize,
    pub grinding_bits: usize,
    pub polynomial_commitment_type: PolynomialCommitmentType,
    pub field_type: FieldType, // LATER: consider infer this from trait
    pub fs_hash: FiatShamirHashType,
    pub gkr_square: bool,
}

pub trait GKRConfig: Default + Clone + Send + 'static {
    /// Field type for the circuit, e.g., M31
    type CircuitField: Field + FieldSerde + Send;

    /// Field type for the challenge, e.g., M31Ext3
    type ChallengeField: BinomialExtensionField<3, BaseField = Self::CircuitField> + Send;

    /// Main field type for the scheme, e.g., SimdM31Ext3
    type Field: BinomialExtensionField<3> + SimdField<Scalar = Self::ChallengeField> + Send;

    /// Field size for the main field
    const FIELD_SIZE: usize;

    const SECURITY_BITS: usize;

    const GRINDING_BITS: usize;

    const POLYNOMIAL_COMMITMENT_TYPE: PolynomialCommitmentType;

    const FIELD_TYPE: FieldType;

    const FS_HASH: FiatShamirHashType;

    const GKR_SQUARE: bool;

    fn challenge_mul_circuit(
        a: &Self::ChallengeField,
        b: &Self::CircuitField,
    ) -> Self::ChallengeField;

    fn field_mul_circuit(a: &Self::Field, b: &Self::CircuitField) -> Self::Field;
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct M31ExtConfig;

impl GKRConfig for M31ExtConfig {
    type CircuitField = M31;

    type ChallengeField = M31Ext3;

    type Field = SimdM31Ext3;

    const FIELD_SIZE: usize = 93;

    const SECURITY_BITS: usize = 100;

    const GRINDING_BITS: usize = 10;

    const POLYNOMIAL_COMMITMENT_TYPE: PolynomialCommitmentType = PolynomialCommitmentType::Raw;

    const FIELD_TYPE: FieldType = FieldType::M31;

    const FS_HASH: FiatShamirHashType = FiatShamirHashType::SHA256;

    const GKR_SQUARE: bool = false;

    fn challenge_mul_circuit(
        a: &Self::ChallengeField,
        b: &Self::CircuitField,
    ) -> Self::ChallengeField {
        a.mul_by_base_field(b)
    }

    fn field_mul_circuit(a: &Self::Field, b: &Self::CircuitField) -> Self::Field {
        // todo! optimize me
        a.scale(&M31Ext3::from(b))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::m31_config()
    }
}

impl Config {
    pub fn m31_config() -> Self {
        println!("===================================================");
        println!("WARNING: Using M31 for testing purposes only.");
        println!("WARNING: Do not use in production.");
        println!("WARNING: M31 may not give you enough soundness.");
        println!("WARNING: consider to use M31Ext3 or Bn254::Fr instead.");
        println!("===================================================");

        let security_bits = 100;
        let grinding_bits = 10;

        let field_size = 31;

        let polynomial_commitment_type = PolynomialCommitmentType::Raw;
        let field_type = FieldType::M31;
        let fs_hash = FiatShamirHashType::SHA256;

        if polynomial_commitment_type == PolynomialCommitmentType::KZG {
            assert_eq!(field_type, FieldType::BN254);
        }

        Config {
            field_size, // update later
            security_bits,
            grinding_bits,
            polynomial_commitment_type,
            field_type,
            fs_hash,
            gkr_square: false,
        }
    }

    // using degree 3 extension of m31
    pub fn m31_ext3_config() -> Self {
        let security_bits = 100;
        let grinding_bits = 10;

        let field_size = 93;

        let polynomial_commitment_type = PolynomialCommitmentType::Raw;
        let field_type = FieldType::M31;
        let fs_hash = FiatShamirHashType::SHA256;

        if polynomial_commitment_type == PolynomialCommitmentType::KZG {
            assert_eq!(field_type, FieldType::BN254);
        }

        Config {
            field_size, // update later
            security_bits,
            grinding_bits,
            polynomial_commitment_type,
            field_type,
            fs_hash,
            gkr_square: false,
        }
    }

    pub fn bn254_config() -> Self {
        let security_bits = 128;
        let grinding_bits = 0;

        let field_size = 254;

        let polynomial_commitment_type = PolynomialCommitmentType::Raw;
        let field_type = FieldType::BN254;
        let fs_hash = FiatShamirHashType::SHA256;

        if polynomial_commitment_type == PolynomialCommitmentType::KZG {
            assert_eq!(field_type, FieldType::BN254);
        }

        Config {
            field_size, // update later
            security_bits,
            grinding_bits,
            polynomial_commitment_type,
            field_type,
            fs_hash,
            gkr_square: false,
        }
    }
}
