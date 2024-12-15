use mersenne31::{M31Ext3, M31};

use crate::{FieldHasherState, PoseidonM31x16Ext3, PoseidonParams};

#[test]
fn test_poseidon_m31() {
    let param = PoseidonParams::<M31, M31Ext3, PoseidonM31x16Ext3>::new();
    let state_elems: [M31; PoseidonM31x16Ext3::STATE_WIDTH] =
        [M31::from(114514); PoseidonM31x16Ext3::STATE_WIDTH];
    let mut state = PoseidonM31x16Ext3::from_elems(&state_elems);

    param.permute(&mut state);

    let expected_elems: Vec<M31> = [
        1044875636, 873839971, 2077250885, 2019235357, 1108829368, 2113595770, 1409201928,
        954157982, 1581312097, 289997806, 1000517632, 514890994, 136890439, 470885623, 1500242465,
        1400794972,
    ]
    .iter()
    .map(|t| From::from(*t))
    .collect();

    assert_eq!(PoseidonM31x16Ext3::from_elems(&expected_elems), state)
}
