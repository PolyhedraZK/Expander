use std::fmt::Debug;

use arith::Field;

pub trait FiatShamirFieldHasher<F: Field>: Clone + Debug + Default {
    /// Name for the field hasher
    const NAME: &'static str;

    /// The state capacity, or how many field elements squeezed in a hash
    const STATE_CAPACITY: usize;

    /// Create a new hash instance.
    fn new() -> Self;

    /// hash a vector of field element and return the hash result
    fn hash_to_state(&self, input: &[F]) -> Vec<F>;
}
