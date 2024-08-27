use std::marker::PhantomData;

use halo2curves::pairing::MultiMillerLoop;

/// Commit to the bi-variate polynomial in its coefficient form.
/// Note that it is in general more efficient to use the lagrange form.
pub struct CoeffFormBiKZG<E: MultiMillerLoop> {
    _phantom: PhantomData<E>,
}
