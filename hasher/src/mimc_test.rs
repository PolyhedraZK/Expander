use arith::{Field, Fr};
use serdes::ExpSerde;

use crate::{FiatShamirHasher, MiMC5FiatShamirHasher};

const MIMC5_BN254_IN: u32 = 123;

// The result is generated by the current version (10/20/2024) of mimc5 itself.
// The point is to pin down a hash output so later we can refer to it.
// There is a similar test in recursion located at recursion/modules/transcript/hash_test.go
const MIMC5_BN254_OUT: [u8; 32] = [
    23, 0, 30, 22, 99, 236, 217, 86, 113, 255, 221, 106, 184, 226, 45, 109, 67, 123, 85, 88, 103,
    54, 177, 150, 88, 18, 208, 172, 76, 143, 30, 5,
];

#[test]
fn check_mimc5_aligned() {
    let mimc = MiMC5FiatShamirHasher::<Fr>::new();
    let input = Fr::from(MIMC5_BN254_IN);
    let mut inputu8 = vec![0u8; Fr::SIZE];
    input.serialize_into(&mut inputu8);
    let mut output = vec![0u8; MiMC5FiatShamirHasher::<Fr>::DIGEST_SIZE];
    mimc.hash(&mut output, &inputu8);

    assert_eq!(output.len(), MiMC5FiatShamirHasher::<Fr>::DIGEST_SIZE);
    assert_eq!(
        output,
        MIMC5_BN254_OUT
    );
}
