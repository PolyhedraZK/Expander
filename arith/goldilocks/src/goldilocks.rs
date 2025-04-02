use std::{
    io::{Read, Write},
    iter::{Product, Sum},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use arith::{field_common, FFTField, Field};
use ethnum::U256;
use rand::RngCore;
use serdes::{ExpSerde, SerdeResult};

use crate::goldilocks::p2_instructions::{assume, branch_hint, reduce128};

// Goldilocks field modulus: 2^64 - 2^32 + 1
pub const GOLDILOCKS_MOD: u64 = 0xFFFFFFFF00000001;
/// 2^32 - 1
pub const EPSILON: u64 = 0xffffffff;

#[inline(always)]
pub(crate) fn mod_reduce_u64(x: u64) -> u64 {
    if x >= GOLDILOCKS_MOD {
        x - GOLDILOCKS_MOD
    } else {
        x
    }
}

#[derive(Debug, Clone, Copy, Default, PartialOrd, Ord)]
pub struct Goldilocks {
    pub v: u64,
}

impl PartialEq for Goldilocks {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        mod_reduce_u64(self.v) == mod_reduce_u64(other.v)
    }
}

impl Eq for Goldilocks {}

field_common!(Goldilocks);

impl ExpSerde for Goldilocks {
    const SERIALIZED_SIZE: usize = 64 / 8;

    #[inline(always)]
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        // normalize the element: both 0 and Modulus are valid internal representations
        let v = mod_reduce_u64(self.v);
        writer.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let mut u = [0u8; Self::SERIALIZED_SIZE];
        reader.read_exact(&mut u)?;
        let mut v = u64::from_le_bytes(u);
        v = mod_reduce_u64(v);
        Ok(Goldilocks { v })
    }
}

impl Field for Goldilocks {
    const NAME: &'static str = "Goldilocks";

    const SIZE: usize = 64 / 8;

    const ZERO: Self = Goldilocks { v: 0 };

    const ONE: Self = Goldilocks { v: 1 };

    const INV_2: Self = Goldilocks {
        v: 0x7FFFFFFF80000001,
    }; // (2^63 - 2^31 + 1)

    const FIELD_SIZE: usize = 64;

    const MODULUS: U256 = U256([GOLDILOCKS_MOD as u128, 0]);

    #[inline(always)]
    fn zero() -> Self {
        Goldilocks { v: 0 }
    }

    #[inline(always)]
    fn one() -> Self {
        Goldilocks { v: 1 }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.v == 0 || self.v == GOLDILOCKS_MOD
    }

    #[inline(always)]
    fn random_unsafe(mut rng: impl RngCore) -> Self {
        rng.next_u64().into()
    }

    #[inline(always)]
    fn random_bool(mut rng: impl RngCore) -> Self {
        (rng.next_u64() & 1).into()
    }

    #[inline(always)]
    fn to_u256(&self) -> U256 {
        U256([self.v as u128, 0])
    }

    #[inline(always)]
    fn from_u256(value: U256) -> Self {
        let v = value % Self::MODULUS;
        Goldilocks { v: v.as_u64() }
    }

    #[inline(always)]
    fn inv(&self) -> Option<Self> {
        self.try_inverse()
    }

    #[inline(always)]
    fn as_u32_unchecked(&self) -> u32 {
        assert!(self.v <= u32::MAX as u64);
        self.v as u32
    }

    #[inline(always)]
    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        let mut v = u64::from_le_bytes(bytes[..8].try_into().unwrap());
        v = mod_reduce_u64(v);
        Goldilocks { v }
    }
}

impl Neg for Goldilocks {
    type Output = Goldilocks;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        Goldilocks::ZERO - self
    }
}

impl From<u32> for Goldilocks {
    #[inline(always)]
    fn from(x: u32) -> Self {
        Goldilocks { v: x as u64 }
    }
}

impl From<u64> for Goldilocks {
    #[inline(always)]
    fn from(x: u64) -> Self {
        Goldilocks {
            v: mod_reduce_u64(x),
        }
    }
}

impl Goldilocks {
    #[inline(always)]
    pub fn exp_power_of_2(&self, power_log: usize) -> Self {
        let mut res = *self;
        for _ in 0..power_log {
            res = res.square();
        }
        res
    }

    /// Squares the base N number of times and multiplies the result by the tail value.
    #[inline(always)]
    fn exp_acc<const N: usize>(base: Goldilocks, tail: Goldilocks) -> Goldilocks {
        base.exp_power_of_2(N) * tail
    }

    #[inline(always)]
    // credit: https://github.com/Plonky3/Plonky3/blob/main/goldilocks/src/goldilocks.rs#L241
    fn try_inverse(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }

        // From Fermat's little theorem, in a prime field `F_p`, the inverse of `a` is `a^(p-2)`.
        //
        // compute a^(p - 2) using 72 multiplications
        // The exponent p - 2 is represented in binary as:
        // 0b1111111111111111111111111111111011111111111111111111111111111111
        // Adapted from: https://github.com/facebook/winterfell/blob/d238a1/math/src/field/f64/mod.rs#L136-L164

        // compute base^11
        let t2 = self.square() * *self;

        // compute base^111
        let t3 = t2.square() * *self;

        // compute base^111111 (6 ones)
        // repeatedly square t3 3 times and multiply by t3
        let t6 = Self::exp_acc::<3>(t3, t3);
        let t60 = t6.square();
        let t7 = t60 * *self;

        // compute base^111111111111 (12 ones)
        // repeatedly square t6 6 times and multiply by t6
        let t12 = Self::exp_acc::<5>(t60, t6);

        // compute base^111111111111111111111111 (24 ones)
        // repeatedly square t12 12 times and multiply by t12
        let t24 = Self::exp_acc::<12>(t12, t12);

        // compute base^1111111111111111111111111111111 (31 ones)
        // repeatedly square t24 6 times and multiply by t6 first. then square t30 and
        // multiply by base
        let t31 = Self::exp_acc::<7>(t24, t7);

        // compute base^111111111111111111111111111111101111111111111111111111111111111
        // repeatedly square t31 32 times and multiply by t31
        let t63 = Self::exp_acc::<32>(t31, t31);

        // compute base^1111111111111111111111111111111011111111111111111111111111111111
        Some(t63.square() * *self)
    }

    #[inline(always)]
    pub fn mul_by_7(&self) -> Self {
        *self * Self { v: 7 }
    }
}

#[inline(always)]
/// credit: plonky2
fn add_internal(a: &Goldilocks, b: &Goldilocks) -> Goldilocks {
    let (sum, over) = a.v.overflowing_add(b.v);
    let (mut sum, over) = sum.overflowing_add((over as u64) * EPSILON);
    if over {
        // NB: self.0 > Self::ORDER && rhs.0 > Self::ORDER is necessary but not sufficient for
        // double-overflow.
        // This assume does two things:
        //  1. If compiler knows that either self.0 or rhs.0 <= ORDER, then it can skip this check.
        //  2. Hints to the compiler how rare this double-overflow is (thus handled better with a
        //     branch).
        assume(a.v > GOLDILOCKS_MOD && b.v > GOLDILOCKS_MOD);
        branch_hint();
        sum += EPSILON; // Cannot overflow.
    }
    Goldilocks { v: sum }
}

#[inline(always)]
fn sub_internal(a: &Goldilocks, b: &Goldilocks) -> Goldilocks {
    let (diff, under) = a.v.overflowing_sub(b.v);
    let (mut diff, under) = diff.overflowing_sub((under as u64) * EPSILON);
    if under {
        // NB: self.0 < EPSILON - 1 && rhs.0 > Self::ORDER is necessary but not sufficient for
        // double-underflow.
        // This assume does two things:
        //  1. If compiler knows that either self.0 >= EPSILON - 1 or rhs.0 <= ORDER, then it can
        //     skip this check.
        //  2. Hints to the compiler how rare this double-underflow is (thus handled better with a
        //     branch).
        assume(a.v < EPSILON - 1 && b.v > GOLDILOCKS_MOD);
        branch_hint();
        diff -= EPSILON; // Cannot underflow.
    }
    Goldilocks { v: diff }
}

#[inline(always)]
fn mul_internal(a: &Goldilocks, b: &Goldilocks) -> Goldilocks {
    reduce128((a.v as u128) * (b.v as u128))
}

impl std::hash::Hash for Goldilocks {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.v);
    }
}

impl FFTField for Goldilocks {
    const TWO_ADICITY: usize = 32; // 2^32 divides p-1

    /// The `2^s` root of unity.
    ///
    /// It can be calculated by exponentiating `Self::MULTIPLICATIVE_GENERATOR` by `t`,
    /// where `t = (modulus - 1) >> Self::S`.
    #[inline(always)]
    fn root_of_unity() -> Self {
        Goldilocks {
            v: 0x185629dcda58878c,
        } // 5 is a primitive root of order 2^32
    }
}

pub(crate) mod p2_instructions {
    //! Credit: the majority of the code is borrowed or inspired from Plonky2 with modifications.

    use std::arch::asm;
    use std::hint::unreachable_unchecked;

    use crate::{Goldilocks, EPSILON};

    /// Reduces to a 64-bit value. The result might not be in canonical form; it could be in between
    /// the field order and `2^64`.
    #[inline]
    pub(crate) fn reduce128(x: u128) -> Goldilocks {
        let (x_lo, x_hi) = split(x); // This is a no-op
        let x_hi_hi = x_hi >> 32;
        let x_hi_lo = x_hi & EPSILON;

        let (mut t0, borrow) = x_lo.overflowing_sub(x_hi_hi);
        if borrow {
            branch_hint(); // A borrow is exceedingly rare. It is faster to branch.
            t0 -= EPSILON; // Cannot underflow.
        }
        let t1 = x_hi_lo * EPSILON;
        let t2 = unsafe { add_no_canonicalize_trashing_input(t0, t1) };
        Goldilocks { v: t2 }
    }

    #[inline(always)]
    pub(super) fn assume(p: bool) {
        debug_assert!(p);
        if !p {
            unsafe {
                unreachable_unchecked();
            }
        }
    }

    /// Try to force Rust to emit a branch. Example:
    ///     if x > 2 {
    ///         y = foo();
    ///         branch_hint();
    ///     } else {
    ///         y = bar();
    ///     }
    /// This function has no semantics. It is a hint only.
    #[inline(always)]
    pub(super) fn branch_hint() {
        unsafe {
            asm!("", options(nomem, nostack, preserves_flags));
        }
    }

    /// Fast addition modulo ORDER for x86-64.
    /// This function is marked unsafe for the following reasons:
    ///   - It is only correct if x + y < 2**64 + ORDER = 0x1ffffffff00000001.
    ///   - It is only faster in some circumstances. In particular, on x86 it overwrites both inputs
    ///     in the registers, so its use is not recommended when either input will be used again.
    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    pub(crate) unsafe fn add_no_canonicalize_trashing_input(x: u64, y: u64) -> u64 {
        let res_wrapped: u64;
        let adjustment: u64;
        asm!(
            "add {0}, {1}",
            // Trick. The carry flag is set iff the addition overflowed.
            // sbb x, y does x := x - y - CF. In our case, x and y are both {1:e}, so it simply does
            // {1:e} := 0xffffffff on overflow and {1:e} := 0 otherwise. {1:e} is the low 32 bits of
            // {1}; the high 32-bits are zeroed on write. In the end, we end up with 0xffffffff in {1}
            // on overflow; this happens be EPSILON.
            // Note that the CPU does not realize that the result of sbb x, x does not actually depend
            // on x. We must write the result to a register that we know to be ready. We have a
            // dependency on {1} anyway, so let's use it.
            "sbb {1:e}, {1:e}",
            inlateout(reg) x => res_wrapped,
            inlateout(reg) y => adjustment,
            options(pure, nomem, nostack),
        );
        assume(x != 0 || (res_wrapped == y && adjustment == 0));
        assume(y != 0 || (res_wrapped == x && adjustment == 0));
        // Add EPSILON == subtract ORDER.
        // Cannot overflow unless the assumption if x + y < 2**64 + ORDER is incorrect.
        res_wrapped + adjustment
    }

    #[inline(always)]
    #[cfg(not(target_arch = "x86_64"))]
    pub(crate) unsafe fn add_no_canonicalize_trashing_input(x: u64, y: u64) -> u64 {
        use crate::EPSILON;

        let (res_wrapped, carry) = x.overflowing_add(y);
        // Below cannot overflow unless the assumption if x + y < 2**64 + ORDER is incorrect.
        res_wrapped + EPSILON * (carry as u64)
    }

    #[inline]
    pub(crate) fn split(x: u128) -> (u64, u64) {
        (x as u64, (x >> 64) as u64)
    }

}
