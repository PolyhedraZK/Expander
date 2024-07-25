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
