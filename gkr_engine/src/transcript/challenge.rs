use arith::Field;

use crate::FieldEngine;

#[derive(Debug, Clone)]
pub struct ExpanderDualVarChallenge<C: FieldEngine> {
    /// random challenge for the x variable
    pub rz_0: Vec<C::ChallengeField>,
    /// random challenge for the y variable
    pub rz_1: Option<Vec<C::ChallengeField>>,
    /// random challenge to merge the SIMD circuit into a single one
    pub r_simd: Vec<C::ChallengeField>,
    /// random challenge to merge the MPI circuit into a single one
    pub r_mpi: Vec<C::ChallengeField>,
}

#[derive(Debug, Clone)]
pub struct ExpanderSingleVarChallenge<C: FieldEngine> {
    /// random challenge for the main body of the circuit
    pub x: Vec<C::ChallengeField>,
    /// random challenge to merge the SIMD circuit into a single one
    pub x_simd: Vec<C::ChallengeField>,
    /// random challenge to merge the MPI circuit into a single one
    pub x_mpi: Vec<C::ChallengeField>,
}

impl<C: FieldEngine> ExpanderSingleVarChallenge<C> {
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

impl<C: FieldEngine> ExpanderDualVarChallenge<C> {
    pub fn new(
        rz_0: Vec<C::ChallengeField>,
        rz_1: Option<Vec<C::ChallengeField>>,
        r_simd: Vec<C::ChallengeField>,
        r_mpi: Vec<C::ChallengeField>,
    ) -> Self {
        Self {
            rz_0,
            rz_1,
            r_simd,
            r_mpi,
        }
    }

    pub fn challenge_x(&self) -> ExpanderSingleVarChallenge<C> {
        ExpanderSingleVarChallenge {
            x: self.rz_0.clone(),
            x_simd: self.r_simd.clone(),
            x_mpi: self.r_mpi.clone(),
        }
    }

    pub fn challenge_y(&self) -> ExpanderSingleVarChallenge<C> {
        assert!(self.rz_1.is_some());

        ExpanderSingleVarChallenge {
            x: self.rz_1.clone().unwrap(),
            x_simd: self.r_simd.clone(),
            x_mpi: self.r_mpi.clone(),
        }
    }
}
