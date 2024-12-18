use arith::ExtensionField;
use mersenne31::{M31Ext3, M31};

use crate::*;

#[test]
fn test_poseidon_hash_m31x16ext3() {
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

#[test]
fn test_poseidon_sponge_m31x16ext3() {
    let mut sponge_bob = PoseidonSponge::<M31, M31Ext3, PoseidonM31x16Ext3>::new();

    (0..PoseidonM31x16Ext3::RATE).for_each(|i| {
        sponge_bob.update(&[M31::from(114514)]);
        assert_eq!(
            sponge_bob.absorbing.len(),
            (i + 1) % PoseidonM31x16Ext3::RATE
        );
    });

    let first_absorbed = sponge_bob.absorbed;
    let (first_squeeze, second_squeeze) = (sponge_bob.squeeze(), sponge_bob.squeeze());
    assert_eq!(first_squeeze, first_absorbed.indexed_digest(0));
    assert_eq!(second_squeeze, first_absorbed.indexed_digest(1));
    assert_eq!(
        sponge_bob.output_index,
        2 * PoseidonM31x16Ext3::OUTPUT_ELEM_DEG
    );

    (0..4).for_each(|i| {
        sponge_bob.update(&[M31::from(893810)]);
        assert_eq!(sponge_bob.output_index, 0);
        assert_eq!(
            sponge_bob.absorbing.len(),
            (i + 1) % PoseidonM31x16Ext3::RATE
        )
    });

    assert_eq!(sponge_bob.absorbed, first_absorbed);

    let third_squeeze = sponge_bob.squeeze();
    assert_ne!(sponge_bob.absorbed, first_absorbed);
    assert_eq!(sponge_bob.output_index, PoseidonM31x16Ext3::OUTPUT_ELEM_DEG);

    let fourth_squeeze = sponge_bob.squeeze();
    assert_eq!(
        sponge_bob.output_index,
        2 * PoseidonM31x16Ext3::OUTPUT_ELEM_DEG
    );

    assert_eq!(third_squeeze, sponge_bob.absorbed.indexed_digest(0));
    assert_eq!(fourth_squeeze, sponge_bob.absorbed.indexed_digest(1));
}
