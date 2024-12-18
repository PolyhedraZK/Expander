use arith::{BN254Fr, FieldSerde};

use crate::{FiatShamirSponge, MiMCFrTranscriptSponge};

const MIMC5_BN254_IN: u32 = 123;

const MIMC5_BN254_ONT: [u8; 32] = [
    23, 0, 30, 22, 99, 236, 217, 86, 113, 255, 221, 106, 184, 226, 45, 109, 67, 123, 85, 88, 103,
    54, 177, 150, 88, 18, 208, 172, 76, 143, 30, 5,
];

#[test]
fn test_mimc_sponge_fr() {
    let mut mimc_sponge = MiMCFrTranscriptSponge::new();
    let input = BN254Fr::from(MIMC5_BN254_IN);
    mimc_sponge.update(&[input]);

    let actual_digest = mimc_sponge.squeeze();
    let expected_digest = BN254Fr::deserialize_from(&MIMC5_BN254_ONT[..]).unwrap();

    assert_eq!(actual_digest, expected_digest);
}
