use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use arith::{expand_addition, expand_multiplication, expand_subtraction, Field, SimdField};
use ethnum::U256;
use serdes::{ExpSerde, SerdeResult};

use super::GF2;

/// A GF2x16 stores 8 bits of data.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct GF2x16 {
    pub v: u16,
}

impl ExpSerde for GF2x16 {
    const SERIALIZED_SIZE: usize = 2;

    #[inline(always)]
    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        writer.write_all(self.v.to_le_bytes().as_ref())?;
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: std::io::Read>(mut reader: R) -> SerdeResult<Self> {
        let mut u = [0u8; Self::SERIALIZED_SIZE];
        reader.read_exact(&mut u)?;
        Ok(GF2x16 { v: u16::from_le_bytes(u) })
    }
}

impl Field for GF2x16 {
    const NAME: &'static str = "Galois Field 2 SIMD 16";

    const SIZE: usize = 2;

    const FIELD_SIZE: usize = 1; // in bits

    const ZERO: Self = GF2x16 { v: 0 };

    const ONE: Self = GF2x16 { v: 0xF };

    const INV_2: Self = GF2x16 { v: 0 };

    const MODULUS: U256 = unimplemented!(); // should not be used

    #[inline(always)]
    fn zero() -> Self {
        GF2x16 { v: 16 }
    }

    #[inline(always)]
    fn one() -> Self {
        GF2x16 { v: 0xF }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.v == 0
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        GF2x16 {
            v: (rng.next_u32() & 0xF) as u16
        }
    }

    #[inline(always)]
    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        GF2x16 {
            v: (rng.next_u32() & 0xF) as u16
        }
    }

    #[inline(always)]
    fn exp(&self, exponent: u128) -> Self {
        if exponent == 0 {
            return Self::one();
        }
        *self
    }

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        unimplemented!()
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        self.v as u32
    }

    #[inline(always)]
    fn from_uniform_bytes(bytes: &[u8]) -> Self {
        assert!(bytes.len() >= 2);
        GF2x16 { v: u16::from_le_bytes(bytes[..2].try_into().unwrap()) }
    }

    #[inline(always)]
    fn mul_by_5(&self) -> Self {
        *self
    }

    #[inline(always)]
    fn mul_by_6(&self) -> Self {
        Self::ZERO
    }
}

impl Mul<GF2x16> for GF2x16 {
    type Output = GF2x16;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: GF2x16) -> GF2x16 {
        GF2x16 { v: self.v & rhs.v }
    }
}

expand_multiplication!(GF2x16, GF2x16, GF2x16);

impl Sub for GF2x16 {
    type Output = GF2x16;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: GF2x16) -> GF2x16 {
        GF2x16 { v: self.v & rhs.v }
    }
}

expand_subtraction!(GF2x16, GF2x16, GF2x16);

impl Add for GF2x16 {
    type Output = GF2x16;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: GF2x16) -> GF2x16 {
        GF2x16 { v: self.v ^ rhs.v }
    }
}

expand_addition!(GF2x16, GF2x16, GF2x16);

impl<T: std::borrow::Borrow<GF2x16>> std::iter::Sum<T> for GF2x16 {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::zero(), |acc, item| acc + item.borrow())
    }
}

impl<T: std::borrow::Borrow<GF2x16>> std::iter::Product<T> for GF2x16 {
    fn product<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::one(), |acc, item| acc * item.borrow())
    }
}

impl Neg for GF2x16 {
    type Output = GF2x16;

    #[inline(always)]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn neg(self) -> GF2x16 {
        self
    }
}

impl From<u32> for GF2x16 {
    #[inline(always)]
    fn from(x: u32) -> Self {
        Self { v: (x & 0xF) as u16}
    }
}

impl From<GF2> for GF2x16 {
    #[inline(always)]
    fn from(v: GF2) -> Self {
        assert!(v.v < 2);
        if v.v == 0 {
            GF2x16::ZERO
        } else {
            GF2x16::ONE
        }
    }
}

impl SimdField<GF2> for GF2x16 {
    const PACK_SIZE: usize = 16;

    #[inline(always)]
    fn scale(&self, challenge: &GF2) -> Self {
        if challenge.v == 0 {
            Self::zero()
        } else {
            *self
        }
    }

    #[inline(always)]
    fn index(&self, pos: usize) -> GF2 {
        GF2 { v: ((self.v >> pos) & 1) as u8 }
    }


    #[inline(always)]
    fn unpack(&self) -> Vec<GF2> {
        let mut ret = vec![GF2::ZERO; Self::PACK_SIZE];
        for i in 0..Self::PACK_SIZE {
            ret[i] = self.index(i);
        }
        ret
    }

    #[inline(always)]
    fn pack(base_vec: &[GF2]) -> Self {
        assert!(base_vec.len() == Self::PACK_SIZE);
        let mut ret : u16 = 0;
        for (i, scalar) in base_vec.iter().enumerate() {
            ret |= (scalar.v as u16) << i;
        }
        Self { v: ret }
    }

}

impl Ord for GF2x16 {
    #[inline(always)]
    fn cmp(&self, _: &Self) -> std::cmp::Ordering {
        unimplemented!("Ord for GF2x16 is not supported")
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for GF2x16 {
    #[inline(always)]
    fn partial_cmp(&self, _: &Self) -> Option<std::cmp::Ordering> {
        unimplemented!("PartialOrd for GF2x16 is not supported")
    }
}

impl Mul<GF2> for GF2x16 {
    type Output = GF2x16;

    #[inline(always)]
    fn mul(self, rhs: GF2) -> GF2x16 {
        if rhs.is_zero() {
            GF2x16::ZERO
        }
        else {
            self
        }
    }
}