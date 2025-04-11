/// macro to implement common arithmetic of field types
#[macro_export]
macro_rules! field_common {
    ($field:ident) => {
        impl Sub<&$field> for $field {
            type Output = $field;

            #[inline]
            fn sub(self, rhs: &$field) -> $field {
                self.sub(*rhs)
            }
        }

        impl Sub<$field> for $field {
            type Output = $field;

            #[inline]
            fn sub(self, rhs: $field) -> $field {
                sub_internal(&self, &rhs)
            }
        }

        impl SubAssign for $field {
            #[inline]
            fn sub_assign(&mut self, rhs: $field) {
                *self = (*self).sub(rhs)
            }
        }

        impl SubAssign<&$field> for $field {
            #[inline]
            fn sub_assign(&mut self, rhs: &$field) {
                *self = (*self).sub(rhs)
            }
        }

        // ========================
        // additions
        // ========================

        impl Add<&$field> for $field {
            type Output = $field;

            #[inline]
            fn add(self, rhs: &$field) -> $field {
                self.add(*rhs)
            }
        }

        impl Add<$field> for $field {
            type Output = $field;

            #[inline]
            fn add(self, rhs: $field) -> $field {
                add_internal(&self, &rhs)
            }
        }

        impl AddAssign for $field {
            #[inline]
            fn add_assign(&mut self, rhs: $field) {
                *self = (*self).add(rhs)
            }
        }

        impl<'b> AddAssign<&'b $field> for $field {
            #[inline]
            fn add_assign(&mut self, rhs: &'b $field) {
                *self = (*self).add(rhs)
            }
        }

        impl<T> Sum<T> for $field
        where
            T: core::borrow::Borrow<Self>,
        {
            fn sum<I>(iter: I) -> Self
            where
                I: Iterator<Item = T>,
            {
                iter.fold(Self::ZERO, |acc, item| acc + item.borrow())
            }
        }

        // ========================
        // multiplications
        // ========================
        impl Mul<$field> for $field {
            type Output = $field;

            #[inline]
            fn mul(self, rhs: $field) -> $field {
                mul_internal(&self, &rhs)
            }
        }

        impl<'b> Mul<&'b $field> for $field {
            type Output = $field;

            #[inline]
            fn mul(self, rhs: &'b $field) -> $field {
                self.mul(*rhs)
            }
        }

        impl Mul<$field> for &$field {
            type Output = $field;

            #[inline(always)]
            fn mul(self, rhs: $field) -> $field {
                *self * rhs
            }
        }

        impl Mul<&$field> for &$field {
            type Output = $field;

            #[inline(always)]
            fn mul(self, rhs: &$field) -> $field {
                *self * *rhs
            }
        }

        impl MulAssign for $field {
            #[inline]
            fn mul_assign(&mut self, rhs: $field) {
                *self = self.clone().mul(rhs)
            }
        }

        impl<'b> MulAssign<&'b $field> for $field {
            #[inline]
            fn mul_assign(&mut self, rhs: &'b $field) {
                *self = self.clone().mul(rhs)
            }
        }

        impl<T> Product<T> for $field
        where
            T: core::borrow::Borrow<Self>,
        {
            fn product<I: Iterator<Item = T>>(iter: I) -> Self {
                iter.fold(Self::one(), |acc, item| acc * item.borrow())
            }
        }
    };
}

#[macro_export]
macro_rules! expand_addition {
    ($lhs:ident, $rhs:ident, $res:ident) => {
        impl Add<&$rhs> for $lhs {
            type Output = $res;

            #[inline]
            fn add(self, rhs: &$rhs) -> $res {
                self.add(*rhs)
            }
        }

        impl AddAssign<$rhs> for $lhs {
            #[inline]
            fn add_assign(&mut self, rhs: $rhs) {
                *self = (*self).add(rhs)
            }
        }

        impl<'b> AddAssign<&'b $rhs> for $lhs {
            #[inline]
            fn add_assign(&mut self, rhs: &'b $rhs) {
                *self = (*self).add(rhs)
            }
        }
    };
}

#[macro_export]
macro_rules! expand_multiplication {
    ($lhs:ident, $rhs:ident, $res:ident) => {
        impl<'b> Mul<&'b $rhs> for $lhs {
            type Output = $res;

            #[inline]
            fn mul(self, rhs: &'b $rhs) -> $res {
                self.mul(*rhs)
            }
        }

        impl Mul<$rhs> for &$lhs {
            type Output = $res;

            #[inline(always)]
            fn mul(self, rhs: $rhs) -> $res {
                *self * rhs
            }
        }

        impl Mul<&$rhs> for &$lhs {
            type Output = $res;

            #[inline(always)]
            fn mul(self, rhs: &$rhs) -> $res {
                *self * *rhs
            }
        }

        impl MulAssign<$rhs> for $lhs {
            #[inline]
            fn mul_assign(&mut self, rhs: $rhs) {
                *self = self.clone().mul(rhs)
            }
        }

        impl<'b> MulAssign<&'b $rhs> for $lhs {
            #[inline]
            fn mul_assign(&mut self, rhs: &'b $rhs) {
                *self = self.clone().mul(rhs)
            }
        }
    };
}

#[macro_export]
macro_rules! expand_subtraction {
    ($lhs:ident, $rhs:ident, $res:ident) => {
        impl Sub<&$rhs> for $lhs {
            type Output = $res;

            #[inline]
            fn sub(self, rhs: &$rhs) -> $res {
                self.sub(*rhs)
            }
        }

        impl SubAssign<$rhs> for $lhs {
            #[inline]
            fn sub_assign(&mut self, rhs: $rhs) {
                *self = (*self).sub(rhs)
            }
        }

        impl<'b> SubAssign<&'b $rhs> for $lhs {
            #[inline]
            fn sub_assign(&mut self, rhs: &'b $rhs) {
                *self = (*self).sub(rhs)
            }
        }
    };
}

#[macro_export]
macro_rules! sum_default {
    ($field:ident) => {
        impl<T> Sum<T> for $field
        where
            T: core::borrow::Borrow<Self>,
        {
            fn sum<I>(iter: I) -> Self
            where
                I: Iterator<Item = T>,
            {
                iter.fold(Self::ZERO, |acc, item| acc + item.borrow())
            }
        }
    };
}

#[macro_export]
macro_rules! product_default {
    ($field:ident) => {
        impl<T> Product<T> for $field
        where
            T: core::borrow::Borrow<Self>,
        {
            fn product<I: Iterator<Item = T>>(iter: I) -> Self {
                iter.fold(Self::one(), |acc, item| acc * item.borrow())
            }
        }
    };
}

#[macro_export]
macro_rules! dummy_multiplication {
    ($lhs:ident, $rhs:ident, $res:ident) => {
        impl Mul<$rhs> for $lhs {
            type Output = $res;

            #[inline]
            fn mul(self, rhs: $rhs) -> $res {
                unreachable!();
            }
        }
    };
}

#[macro_export]
macro_rules! dummy_addition {
    ($lhs:ident, $rhs:ident, $res:ident) => {
        impl Add<$rhs> for $lhs {
            type Output = $res;

            #[inline]
            fn add(self, rhs: $rhs) -> $res {
                unreachable!();
            }
        }
    };
}

#[macro_export]
macro_rules! dummy_from {
    ($source:ident, $target:ident) => {
        impl From<$source> for $target {
            #[inline]
            fn from(x: $source) -> $target {
                unreachable!();
            }
        }
    };
}

#[macro_export]
macro_rules! unit_simd {
    ($field:ident) => {
        impl SimdField<$field> for $field {
            const PACK_SIZE: usize = 1;

            /// scale self with the challenge
            fn scale(&self, challenge: &$field) -> Self {
                *self * challenge
            }

            /// unpack into a vector.
            fn unpack(&self) -> Vec<$field> {
                vec![*self]
            }

            fn pack(base_vec: &[$field]) -> Self {
                base_vec[0]
            }
        }
    };
}

#[macro_export]
macro_rules! pack_add_assign_default {
    ($field:ident, $scalar:ident, $simd:ident) => {
        impl PackAddAssign<$scalar, $simd> for $field {
            #[inline(always)]
        }
    };
}