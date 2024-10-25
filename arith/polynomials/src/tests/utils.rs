use arith::Field;
use ark_std::test_rng;
use halo2curves::bn256::Fr;

use crate::batch_inversion_in_place;

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
