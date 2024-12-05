use arith::{Field, SimdField};
use communicator::{MPICommunicator, ExpanderComm};
use transcript::Transcript;

// #[inline(always)]
// pub(crate) fn unpack_and_sum<F: SimdField>(p: &F) -> F::Scalar {
//     p.unpack().into_iter().sum()
// }

#[inline(always)]
/// Input
/// - a SIMD field, denoted by p := [p0, ... pn]
/// - a vector of coefficients, denoted by coef := [c0, ... cn]
///
/// Output
/// - p0 * c0 + ... + pn * cn
pub fn unpack_and_combine<F: SimdField>(p: &F, coef: &[F::Scalar]) -> F::Scalar {
    let p_unpacked = p.unpack();
    p_unpacked
        .into_iter()
        .zip(coef)
        .map(|(p_i, coef_i)| p_i * coef_i)
        .sum()
}

/// Transcript IO between sumcheck steps
#[inline]
pub fn transcript_io<F, T>(mpi_comm: &MPICommunicator, ps: &[F], transcript: &mut T) -> F
where
    F: Field,
    T: Transcript<F>,
{
    assert!(ps.len() == 3 || ps.len() == 4); // 3 for x, y; 4 for simd var
    for p in ps {
        transcript.append_field_element(p);
    }
    let mut r = transcript.generate_challenge_field_element();
    mpi_comm.root_broadcast_f(&mut r);
    r
}
