//! Definitions of Montgomery field elements.
// The mojarity of the code are taken from Plonky3.
// Ideally we want to simply import or wrap plonky3's field implementation.
// But plonky3 has a feature flag on avx256/avx512 where expander
// decides whether to use avx256 or avx512 at compile time.
//
// So we re-implement the field in our own crate.

use std::{cmp::Ordering, io::{Read, Write}, marker::PhantomData, ops::Neg};

mod param;
use ethnum::U256;
pub use param::*;

mod utils;
use serdes::{ExpSerde, SerdeResult};
pub use utils::*;

use crate::Field;

#[cfg(target_arch = "aarch64")]
mod neon;
// #[cfg(target_arch = "aarch64")]
// pub type BabyBearx16 = babybear_neon::NeonBabyBear;

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
mod avx512;
// #[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
// pub type PackedMontyParameters = avx512::PackedMontyParameters;

// Fallback, use avx2
#[cfg(all(target_arch = "x86_64", not(target_feature = "avx512f")))]
mod avx256;
// #[cfg(all(target_arch = "x86_64", not(target_feature = "avx512f")))]
// pub type BabyBearx16 = babybear_avx256::AVXBabyBear;

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
#[repr(transparent)] // Packed field implementations rely on this!
pub struct MontyField31<MP: MontyParameters> {
    /// The MONTY form of the field element, saved as a positive integer less than `P`.
    ///
    /// This is `pub(crate)` for tests and delayed reduction strategies. If you're accessing
    /// `value` outside of those, you're likely doing something fishy.
    pub value: u32,
    _phantom: PhantomData<MP>,
}

impl<MP: MontyParameters> MontyField31<MP> {
    /// The standard way to crate a new element.
    /// Note that new converts the input into MONTY form so should be avoided in performance
    /// critical implementations.
    #[inline(always)]
    pub const fn new(value: u32) -> Self {
        Self {
            value: to_monty::<MP>(value),
            _phantom: PhantomData,
        }
    }

    /// Create a new field element from something already in MONTY form.
    /// This is `pub(crate)` for tests and delayed reduction strategies. If you're using it outside
    /// of those, you're likely doing something fishy.
    #[inline(always)]
    pub(crate) const fn new_monty(value: u32) -> Self {
        Self {
            value,
            _phantom: PhantomData,
        }
    }

    /// Produce a u32 in range [0, P) from a field element corresponding to the true value.
    #[inline(always)]
    pub(crate) const fn to_u32(elem: &Self) -> u32 {
        from_monty::<MP>(elem.value)
    }

    /// Convert a constant u32 array into a constant array of field elements.
    /// Constant version of array.map(MontyField31::new).
    #[inline]
    pub const fn new_array<const N: usize>(input: [u32; N]) -> [Self; N] {
        let mut output = [Self::new_monty(0); N];
        let mut i = 0;
        while i < N {
            output[i] = Self::new(input[i]);
            i += 1;
        }
        output
    }

    /// Convert a constant 2d u32 array into a constant 2d array of field elements.
    /// Constant version of array.map(MontyField31::new_array).
    #[inline]
    pub const fn new_2d_array<const N: usize, const M: usize>(
        input: [[u32; N]; M],
    ) -> [[Self; N]; M] {
        let mut output = [[Self::new_monty(0); N]; M];
        let mut i = 0;
        while i < M {
            output[i] = Self::new_array(input[i]);
            i += 1;
        }
        output
    }
}


impl<MP: MontyParameters> Neg for MontyField31<MP> {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self::Output {
       Self::new(MP::PRIME - self.value)
    }
}

impl<MP: MontyParameters> From<u32> for MontyField31<MP> {
    #[inline(always)]
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl<MP: MontyParameters> ExpSerde for MontyField31<MP> {
    const SERIALIZED_SIZE: usize = 32 / 8;

    #[inline(always)]
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        // Note: BabyBear's impl of as_u32_unchecked() converts to canonical form
        writer.write_all(self.value.to_le_bytes().as_ref())?;
        Ok(())
    }

    /// Note: This function performs modular reduction on inputs and
    /// converts from canonical to Montgomery form.
    #[inline(always)]
    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let mut u = [0u8; Self::SERIALIZED_SIZE];
        reader.read_exact(&mut u)?;
        let v = u32::from_le_bytes(u);
        Ok(Self::from(v))
    }
}

impl<MP: FieldParameters> PartialOrd for MontyField31<MP> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}


impl<MP: FieldParameters> Ord for MontyField31<MP> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}



impl<MP: FieldParameters> Field for MontyField31<MP> {
    const NAME: &'static str = "Monty Field";

    const SIZE: usize = 32 / 8;

    const FIELD_SIZE: usize = 32;

    const ZERO: Self = Self::new(0);

    const ONE: Self = Self::new(1);

    const MODULUS: U256 = U256([MP::PRIME as u128, 0]);

    // See test below
    const INV_2: Self = Self::new(1006632961);

    #[inline(always)]
    fn zero() -> Self {
        Self::ZERO
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }

    #[inline(always)]
    fn one() -> Self {
        Self::ONE
    }

    /// Uses rejection sampling to avoid bias.
    fn random_unsafe(mut rng: impl rand::RngCore) -> Self {
        Self::new(rng.next_u32())
    }

    fn random_bool(mut rng: impl rand::RngCore) -> Self {
        (rng.next_u32() & 1).into()
    }

    fn inv(&self) -> Option<Self> {
        <MP as FieldParameters>::try_inverse(self)
    }

    /// Converts to canonical form.
    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        to_monty::<MP>(self.value)
    }

    #[inline(always)]
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        // Note: From<u32> performs modular reduction
        u32::from_le_bytes(bytes[..4].try_into().unwrap()).into()
    }
}

