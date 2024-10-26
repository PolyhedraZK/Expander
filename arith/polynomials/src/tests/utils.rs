use arith::Field;
use ark_std::test_rng;
use halo2curves::bn256::Fr;

use crate::{batch_inversion_in_place, gaussian_elimination};

#[test]
fn test_batch_inversion() {
    let mut rng = test_rng();

    // Test case with multiple elements
    let mut values = (0..12)
        .map(|_| Fr::random_unsafe(&mut rng))
        .collect::<Vec<_>>();
    let original = values.clone();

    batch_inversion_in_place(&mut values);

    // Verify each element is inverted correctly by multiplying with original
    values.iter().zip(original.iter()).for_each(|(inv, orig)| {
        assert_eq!(inv * orig, Fr::one(),);
    });
}

#[test]
fn test_gauss() {
    let mut matrix: Vec<Vec<Fr>> = vec![
        vec![Fr::one(), Fr::zero(), Fr::zero(), Fr::from(2u64)],
        vec![Fr::one(), Fr::one(), Fr::one(), Fr::from(17u64)],
        vec![Fr::one(), Fr::from(2u64), Fr::from(4u64), Fr::from(38u64)],
    ];
    let result = vec![Fr::from(2u64), Fr::from(12u64), Fr::from(3u64)];
    assert_eq!(gaussian_elimination(&mut matrix), result);
}
