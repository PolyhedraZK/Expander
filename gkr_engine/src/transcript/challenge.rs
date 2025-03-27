use arith::Field;

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

impl<C: FieldEngine> ExpanderChallenge<C> {
    pub fn local_xs(&self) -> Vec<C::ChallengeField> {
        let mut local_xs = vec![C::ChallengeField::ZERO; self.x_simd.len() + self.x.len()];
        local_xs[..self.x_simd.len()].copy_from_slice(&self.x_simd);
        local_xs[self.x_simd.len()..].copy_from_slice(&self.x);
        local_xs
    }

    pub fn global_xs(&self) -> Vec<C::ChallengeField> {
        let mut global_xs = vec![C::ChallengeField::ZERO; self.num_vars()];
        global_xs[..self.x_simd.len() + self.x.len()].copy_from_slice(&self.local_xs());
        global_xs[self.x_simd.len() + self.x.len()..].copy_from_slice(&self.x_mpi);
        global_xs
    }

    pub fn num_vars(&self) -> usize {
        self.x.len() + self.x_simd.len() + self.x_mpi.len()
    }
}
