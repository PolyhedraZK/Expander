#[derive(Debug, Clone, PartialEq, Default)]
pub enum FiatShamirHashType {
    #[default]
    SHA256,
    Keccak256,
    Poseidon,
    Animoe,
    MIMC5, // Note: use MIMC5 for bn254 ONLY
}

/// Fiat Shamir hash type
const FIAT_SHAMIR_HASH: FiatShamirHashType;