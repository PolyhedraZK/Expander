use arith::{BN254Fr, FiatShamirFieldHasher, FieldSerde};
use halo2curves::bn256::Fr;

use crate::MiMC5FiatShamirHasher;

const MIMC5_BN254_IN: u32 = 123;

// The result is generated by the currect version (10/20/2024) of mimc5 itself.
// The point is to pin down a hash output so later we can refer to it.
// There is a similar test in recursion located at recursion/modules/transcript/hash_test.go
const MIMC5_BN254_ONT: [u8; 32] = [
    23, 0, 30, 22, 99, 236, 217, 86, 113, 255, 221, 106, 184, 226, 45, 109, 67, 123, 85, 88, 103,
    54, 177, 150, 88, 18, 208, 172, 76, 143, 30, 5,
];

#[test]
fn check_mimc5_aligned() {
    let mimc = MiMC5FiatShamirHasher::<Fr>::new();
    let input = BN254Fr::from(MIMC5_BN254_IN);
    let output = mimc.hash(&[input]);

    assert_eq!(output.len(), MiMC5FiatShamirHasher::<Fr>::STATE_CAPACITY);
    assert_eq!(
        output[0],
        Fr::deserialize_from(&MIMC5_BN254_ONT[..]).unwrap()
    );
}
