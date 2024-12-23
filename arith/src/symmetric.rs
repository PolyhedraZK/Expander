use std::fmt::Debug;

use crate::Field;

pub trait FiatShamirFieldHash<F: Field>: Clone + Debug + Default {
    // TODO(HS) Hash name

    // TODO(HS) Hash state?

    /// Create a new hash instance.
    fn new() -> Self;

    /// hash a vector of field element and return the hash result
    fn hash(&self, input: &[F]) -> F;
}
