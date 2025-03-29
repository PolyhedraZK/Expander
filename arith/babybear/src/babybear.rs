use std::{
    io::{Read, Write},
    iter::{Product, Sum},
    mem::transmute,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::{field_common, Field};
use ethnum::U256;
use p3_baby_bear::BabyBear as P3Babybear;
use p3_field::{Field as P3Field, PrimeCharacteristicRing, PrimeField32};
use rand::RngCore;
use serdes::{ExpSerde, SerdeResult};

// Babybear field modulus: 2^31 - 2^27 + 1
pub const BABYBEAR_MOD: u64 = 0x78000001;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BabyBear(pub P3Babybear);

field_common!(BabyBear);

impl ExpSerde for BabyBear {
    const SERIALIZED_SIZE: usize = 32 / 8;

    #[inline(always)]
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        Ok(writer.write_all(self.0.to_unique_u32().to_le_bytes().as_ref())?)
    }

    #[inline(always)]
    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        // we want to avoid converting from and to Montgomery form
        // the inner value are not exposed so we have to use unsafe transmute
        let mut data = [0u8; 4];
        reader.read_exact(&mut data)?;
        Ok(Self(unsafe {
            transmute::<u32, P3Babybear>(u32::from_le_bytes(data))
        }))
    }
}

impl Field for BabyBear {
    const NAME: &'static str = "BabyBear";

    const SIZE: usize = 64 / 8;

    const ZERO: Self = BabyBear(P3Babybear::ZERO);

    const ONE: Self = BabyBear(P3Babybear::ONE);

    // 1/2 % r = 3c000001
    const INV_2: Self = BabyBear(P3Babybear::new(0x3c000001));

    const FIELD_SIZE: usize = 32;

    const MODULUS: U256 = U256([BABYBEAR_MOD as u128, 0]);

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl RngCore) -> Self {
        Self(P3Babybear::new(rng.next_u32()))
    }

    #[inline(always)]
    fn random_bool(mut rng: impl RngCore) -> Self {
        Self(P3Babybear::new(rng.next_u32() & 1))
    }

    #[inline(always)]
    fn to_u256(&self) -> U256 {
        U256([self.0.as_canonical_u32() as u128, 0])
    }

    #[inline(always)]
    fn from_u256(value: U256) -> Self {
        assert!(value < Self::MODULUS);
        // TODO: this is a hack to get the low 64 bits of the u256
        // TODO: we should remove the assumption that the top bits are 0s
        let (_high, low) = value.into_words();
        let v = low as u32;
        Self(P3Babybear::new(v))
    }

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }
        Some(Self(self.0.inverse()))
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        unimplemented!()
    }

    #[inline(always)]
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        let v = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        Self(P3Babybear::new(v))
    }
}

impl Neg for BabyBear {
    type Output = BabyBear;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        BabyBear(self.0.neg())
    }
}

impl From<u32> for BabyBear {
    #[inline(always)]
    fn from(x: u32) -> Self {
        BabyBear(P3Babybear::new(x))
    }
}

#[inline(always)]
fn add_internal(a: &BabyBear, b: &BabyBear) -> BabyBear {
    BabyBear(a.0 + b.0)
}

#[inline(always)]
fn sub_internal(a: &BabyBear, b: &BabyBear) -> BabyBear {
    BabyBear(a.0 - b.0)
}

#[inline(always)]
fn mul_internal(a: &BabyBear, b: &BabyBear) -> BabyBear {
    BabyBear(a.0 * b.0)
}
