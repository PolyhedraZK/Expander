use std::{
    io::{Read, Write},
    iter::{Product, Sum},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::{field_common, Field, FieldParameters, MontyField31, MontyParameters, PackedMontyParameters};
use ark_std::Zero;
use ethnum::U256;
use serdes::{ExpSerde, SerdeResult};


/// The prime field `2^31 - 2^27 + 1`, a.k.a. the Baby Bear field.
pub type BabyBear = MontyField31<BabyBearParameters>;




#[derive(Copy, Clone, Default, Debug, Eq, Hash, PartialEq)]
pub struct BabyBearParameters;

impl MontyParameters for BabyBearParameters {
    /// The Baby Bear prime: 2^31 - 2^27 + 1.
    /// This is the unique 31-bit prime with the highest possible 2 adicity (27).
    const PRIME: u32 = 0x78000001;

    const MONTY_BITS: u32 = 32;
    const MONTY_MU: u32 = 0x88000001;
}

impl PackedMontyParameters for BabyBearParameters {}
// impl PackedMontyParameters for BabyBearParameters {}

// impl BarrettParameters for BabyBearParameters {}

impl FieldParameters for BabyBearParameters {
    const MONTY_GEN: BabyBear = BabyBear::new(31);

    fn try_inverse(p1: &BabyBear) -> Option<BabyBear> {
        if p1.is_zero() {
            return None;
        }

        // From Fermat's little theorem, in a prime field `F_p`, the inverse of `a` is `a^(p-2)`.
        // Here p-2 = 2013265919 = 1110111111111111111111111111111_2.
        // Uses 30 Squares + 7 Multiplications => 37 Operations total.

        let p100000000 = p1.exp_power_of_2(8);
        let p100000001 = p100000000 * p1;
        let p10000000000000000 = p100000000.exp_power_of_2(8);
        let p10000000100000001 = p10000000000000000 * p100000001;
        let p10000000100000001000 = p10000000100000001.exp_power_of_2(3);
        let p1000000010000000100000000 = p10000000100000001000.exp_power_of_2(5);
        let p1000000010000000100000001 = p1000000010000000100000000 * p1;
        let p1000010010000100100001001 = p1000000010000000100000001 * p10000000100000001000;
        let p10000000100000001000000010 = p1000000010000000100000001.square();
        let p11000010110000101100001011 = p10000000100000001000000010 * p1000010010000100100001001;
        let p100000001000000010000000100 = p10000000100000001000000010.square();
        let p111000011110000111100001111 =
            p100000001000000010000000100 * p11000010110000101100001011;
        let p1110000111100001111000011110000 = p111000011110000111100001111.exp_power_of_2(4);
        let p1110111111111111111111111111111 =
            p1110000111100001111000011110000 * p111000011110000111100001111;

        Some(p1110111111111111111111111111111)
    }
}

// impl RelativelyPrimePower<7> for BabyBearParameters {
//     /// In the field `BabyBear`, `a^{1/7}` is equal to a^{1725656503}.
//     ///
//     /// This follows from the calculation `7 * 1725656503 = 6*(2^31 - 2^27) + 1 = 1 mod (p - 1)`.
//     fn exp_root_d<R: PrimeCharacteristicRing>(val: R) -> R {
//         exp_1725656503(val)
//     }
// }

// impl TwoAdicData for BabyBearParameters {
//     const TWO_ADICITY: usize = 27;

//     type ArrayLike = &'static [BabyBear];

//     const TWO_ADIC_GENERATORS: Self::ArrayLike = &BabyBear::new_array([
//         0x1, 0x78000000, 0x67055c21, 0x5ee99486, 0xbb4c4e4, 0x2d4cc4da, 0x669d6090, 0x17b56c64,
//         0x67456167, 0x688442f9, 0x145e952d, 0x4fe61226, 0x4c734715, 0x11c33e2a, 0x62c3d2b1,
//         0x77cad399, 0x54c131f4, 0x4cabd6a6, 0x5cf5713f, 0x3e9430e8, 0xba067a3, 0x18adc27d,
//         0x21fd55bc, 0x4b859b3d, 0x3bd57996, 0x4483d85a, 0x3a26eef8, 0x1a427a41,
//     ]);

//     const ROOTS_8: Self::ArrayLike = &BabyBear::new_array([0x1, 0x5ee99486, 0x67055c21,
// 0xc9ea3ba]);     const INV_ROOTS_8: Self::ArrayLike =
//         &BabyBear::new_array([0x1, 0x6b615c47, 0x10faa3e0, 0x19166b7b]);

//     const ROOTS_16: Self::ArrayLike = &BabyBear::new_array([
//         0x1, 0xbb4c4e4, 0x5ee99486, 0x4b49e08, 0x67055c21, 0x5376917a, 0xc9ea3ba, 0x563112a7,
//     ]);
//     const INV_ROOTS_16: Self::ArrayLike = &BabyBear::new_array([
//         0x1, 0x21ceed5a, 0x6b615c47, 0x24896e87, 0x10faa3e0, 0x734b61f9, 0x19166b7b, 0x6c4b3b1d,
//     ]);
// }

// impl BinomialExtensionData<4> for BabyBearParameters {
//     const W: BabyBear = BabyBear::new(11);
//     const DTH_ROOT: BabyBear = BabyBear::new(1728404513);
//     const EXT_GENERATOR: [BabyBear; 4] = BabyBear::new_array([8, 1, 0, 0]);
//     const EXT_TWO_ADICITY: usize = 29;

//     type ArrayLike = [[BabyBear; 4]; 2];
//     const TWO_ADIC_EXTENSION_GENERATORS: Self::ArrayLike =
//         BabyBear::new_2d_array([[0, 0, 1996171314, 0], [0, 0, 0, 124907976]]);
// }

// impl BinomialExtensionData<5> for BabyBearParameters {
//     const W: BabyBear = BabyBear::new(2);
//     const DTH_ROOT: BabyBear = BabyBear::new(815036133);
//     const EXT_GENERATOR: [BabyBear; 5] = BabyBear::new_array([8, 1, 0, 0, 0]);
//     const EXT_TWO_ADICITY: usize = 27;

//     type ArrayLike = [[BabyBear; 5]; 0];
//     const TWO_ADIC_EXTENSION_GENERATORS: Self::ArrayLike = [];
// }

// field_common!(BabyBear);

// #[inline(always)]
// fn add_internal(a: &BabyBear, b: &BabyBear) -> BabyBear {
//     BabyBear(a.0 + b.0)
// }

// #[inline(always)]
// fn sub_internal(a: &BabyBear, b: &BabyBear) -> BabyBear {
//     BabyBear(a.0 - b.0)
// }

// #[inline(always)]
// fn mul_internal(a: &BabyBear, b: &BabyBear) -> BabyBear {
//     BabyBear(a.0 * b.0)
// }

#[test]
fn baby_bear_two_inverse() {
    let two = BabyBear::new(2);
    let two_inverse = BabyBearParameters::try_inverse(&two).unwrap();
    // Display impl converts to canonical form
    println!("2^-1 (canonical form): {two_inverse}");

    // Check correctness
    let two = BabyBear::new(2);
    let two_inverse_canonical: u32 = 1006632961;
    let two_inverse = BabyBear::new(two_inverse_canonical);
    let one = BabyBear::ONE;
    assert_eq!(one, two * two_inverse)
}

#[test]
fn test_exponentiation() {
    use rand::{rngs::OsRng, Rng};
    let mut rng = OsRng;

    for _ in 0..1000 {
        // Use a small base to avoid overflow
        let base_u32: u32 = rng.gen_range(0..=10);
        let base = BabyBear::new(base_u32);
        // Use a small exponent to avoid overflow
        let exponent: u32 = rng.gen_range(0..=5);
        let expected_result = BabyBear::new(base_u32.pow(exponent));
        assert_eq!(base.exp(exponent as u128), expected_result);
    }
}
