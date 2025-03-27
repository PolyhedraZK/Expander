use crate::FieldEngine;

#[derive(Debug, Clone)]
pub struct ExpanderChallenge<C: FieldEngine> {
    /// random challenge for the main body of the circuit
    pub x: Vec<C::ChallengeField>,
    /// random challenge to merge the SIMD circuit into a single one
    pub x_simd: Vec<C::ChallengeField>,
    /// random challenge to merge the MPI circuit into a single one
    pub x_mpi: Vec<C::ChallengeField>,
}
