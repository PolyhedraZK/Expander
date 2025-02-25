use std::io::{Read, Write};

use halo2curves::{
    bn256::{Fr, G1Affine, G2Affine},
    ff::{Field as Halo2Field, FromUniformBytes, PrimeField},
    group::GroupEncoding,
};
use rand::RngCore;

use crate::serde::{FieldSerdeError, FieldSerdeResult};
use crate::{ExtensionField, Field, FieldForECC, FieldSerde, SimdField};

const MODULUS: ethnum::U256 = ethnum::U256([
    0x2833e84879b9709143e1f593f0000001,
    0x30644e72e131a029b85045b68181585d,
]);

pub use halo2curves::bn256::Fr as BN254Fr;

impl Field for Fr {
    /// name
    const NAME: &'static str = "bn254 scalar field";

    /// size required to store the data
    const SIZE: usize = 32;

    const FIELD_SIZE: usize = 256;

    /// zero
    const ZERO: Self = Fr::zero();

    /// One
    const ONE: Self = Fr::one();

    /// Inverse of 2
    const INV_2: Self = Fr::TWO_INV;

    // ====================================
    // constants
    // ====================================
    /// Zero element
    #[inline(always)]
    fn zero() -> Self {
        Fr::zero()
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        *self == Fr::zero()
    }

    /// Identity element
    #[inline(always)]
    fn one() -> Self {
        Fr::one()
    }

    // ====================================
    // generators
    // ====================================
    /// create a random element from rng.
    /// test only -- the output may not be uniformly random.
    #[inline(always)]
    fn random_unsafe(rng: impl RngCore) -> Self {
        Fr::random(rng)
    }

    /// create a random boolean element from rng
    #[inline(always)]
    fn random_bool(mut rng: impl RngCore) -> Self {
        Self::from((rng.next_u32() & 1) as u64)
    }

    // ====================================
    // arithmetics
    // ====================================
    /// Squaring
    #[inline(always)]
    fn square(&self) -> Self {
        *self * *self
    }

    /// Doubling
    #[inline(always)]
    fn double(&self) -> Self {
        *self + *self
    }

    /// Exp
    fn exp(&self, _exponent: u128) -> Self {
        unimplemented!()
    }

    /// find the inverse of the element; return None if not exist
    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        self.invert().into()
    }

    /// expose the element as u32.
    fn as_u32_unchecked(&self) -> u32 {
        todo!()
    }

    // TODO: better implementation
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        <Fr as FromUniformBytes<64>>::from_uniform_bytes(
            &[bytes.as_slice(), [0u8; 32].as_slice()]
                .concat()
                .try_into()
                .unwrap(),
        )
    }
}

impl FieldForECC for Fr {
    const MODULUS: ethnum::U256 = MODULUS;

    fn from_u256(x: ethnum::U256) -> Self {
        Fr::from_bytes(&(x % MODULUS).to_le_bytes()).unwrap()
    }
    fn to_u256(&self) -> ethnum::U256 {
        ethnum::U256::from_le_bytes(self.to_bytes())
    }
}

impl SimdField for Fr {
    type Scalar = Self;

    #[inline(always)]
    fn scale(&self, challenge: &Self::Scalar) -> Self {
        self * challenge
    }

    #[inline(always)]
    fn pack(base_vec: &[Self::Scalar]) -> Self {
        assert!(base_vec.len() == 1);
        base_vec[0]
    }

    #[inline(always)]
    fn unpack(&self) -> Vec<Self::Scalar> {
        vec![*self]
    }

    const PACK_SIZE: usize = 1;
}

impl FieldSerde for Fr {
    const SERIALIZED_SIZE: usize = 32;

    #[inline(always)]
    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        writer.write_all(self.to_bytes().as_ref())?;
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let mut buffer = [0u8; Self::SERIALIZED_SIZE];
        reader.read_exact(&mut buffer)?;
        match Fr::from_bytes(&buffer).into_option() {
            Some(v) => Ok(v),
            None => Err(FieldSerdeError::DeserializeError),
        }
    }
}

impl FieldSerde for G1Affine {
    const SERIALIZED_SIZE: usize = 32;

    fn serialize_into<W: Write>(&self, writer: W) -> FieldSerdeResult<()> {
        let bytes = self.to_bytes().as_ref().to_vec();
        bytes.serialize_into(writer)
    }

    fn deserialize_from<R: Read>(reader: R) -> FieldSerdeResult<Self> {
        let bytes: Vec<u8> = Vec::deserialize_from(reader)?;
        if bytes.len() != Self::SERIALIZED_SIZE {
            return Err(FieldSerdeError::DeserializeError);
        }

        let mut encoding = <Self as GroupEncoding>::Repr::default();
        encoding.as_mut().copy_from_slice(bytes.as_ref());
        match G1Affine::from_bytes(&encoding).into_option() {
            Some(a) => Ok(a),
            None => Err(FieldSerdeError::DeserializeError),
        }
    }
}

impl FieldSerde for G2Affine {
    const SERIALIZED_SIZE: usize = 64;

    fn serialize_into<W: Write>(&self, writer: W) -> FieldSerdeResult<()> {
        let bytes = self.to_bytes().as_ref().to_vec();
        bytes.serialize_into(writer)
    }

    fn deserialize_from<R: Read>(reader: R) -> FieldSerdeResult<Self> {
        let bytes: Vec<u8> = Vec::deserialize_from(reader)?;
        if bytes.len() != Self::SERIALIZED_SIZE {
            return Err(FieldSerdeError::DeserializeError);
        }

        let mut encoding = <Self as GroupEncoding>::Repr::default();
        encoding.as_mut().copy_from_slice(bytes.as_ref());
        match G2Affine::from_bytes(&encoding).into_option() {
            Some(a) => Ok(a),
            None => Err(FieldSerdeError::DeserializeError),
        }
    }
}

impl ExtensionField for Fr {
    const DEGREE: usize = 1;

    /// Extension Field over X-1 which is self
    const W: u32 = 1;

    // placeholder, doesn't make sense for Fr
    const X: Self = Fr::zero();

    /// Base field for the extension
    type BaseField = Self;

    /// Multiply the extension field with the base field
    fn mul_by_base_field(&self, base: &Self::BaseField) -> Self {
        self * base
    }

    /// Add the extension field with the base field
    fn add_by_base_field(&self, base: &Self::BaseField) -> Self {
        self + base
    }

    /// Multiply the extension field by x, i.e, 0 + x + 0 x^2 + 0 x^3 + ...
    fn mul_by_x(&self) -> Self {
        unimplemented!("mul_by_x for Fr doesn't make sense")
    }

    /// Construct a new instance of extension field from coefficients
    fn from_limbs(limbs: &[Self::BaseField]) -> Self {
        if limbs.len() < Self::DEGREE {
            Self::zero()
        } else {
            limbs[0]
        }
    }

    /// Extract polynomial field coefficients from the extension field instance
    fn to_limbs(&self) -> Vec<Self::BaseField> {
        vec![*self]
    }
}
