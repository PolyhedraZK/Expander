use arith::ExtensionField;
use mersenne31::{M31Ext3, M31};

use crate::{FieldHasher, FieldHasherState, PoseidonM31x16Ext3, PoseidonParams};

#[test]
fn test_poseidon_m31() {
    let param = PoseidonParams::<M31, M31Ext3, PoseidonM31x16Ext3>::new();
    let state_elems: [M31; PoseidonM31x16Ext3::STATE_WIDTH] =
        [M31::from(114514); PoseidonM31x16Ext3::STATE_WIDTH];

    let actual_output = param.hash(&state_elems);

    let expected_output = M31Ext3::from_limbs(&[
        M31::from(1044875636),
        M31::from(873839971),
        M31::from(2077250885),
    ]);

    assert_eq!(actual_output, expected_output)
}
