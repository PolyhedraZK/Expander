use std::mem::transmute;

use mersenne31::{M31x16, M31};

use crate::{PoseidonParams, PoseidonState};

#[test]
fn test_poseidon_m31() {
    let param = PoseidonParams::<M31, M31x16>::new();
    let mut state = unsafe { transmute::<[u32; M31x16::STATE_WIDTH], M31x16>([114514; 16]) };

    param.permute(&mut state);

    let expected = unsafe {
        transmute::<[u32; M31x16::STATE_WIDTH], M31x16>([
            1044875636, 873839971, 2077250885, 2019235357, 1108829368, 2113595770, 1409201928,
            954157982, 1581312097, 289997806, 1000517632, 514890994, 136890439, 470885623,
            1500242465, 1400794972,
        ])
    };
    assert_eq!(expected, state)
}
