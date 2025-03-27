use arith::Field;

use crate::FieldEngine;

#[derive(Debug, Clone, Default)]
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

#[derive(Debug, Clone, Default)]
pub struct ExpanderSingleVarChallenge<C: FieldEngine> {
    /// random challenge for the main body of the circuit
    pub rz: Vec<C::ChallengeField>,
    /// random challenge to merge the SIMD circuit into a single one
    pub r_simd: Vec<C::ChallengeField>,
    /// random challenge to merge the MPI circuit into a single one
    pub r_mpi: Vec<C::ChallengeField>,
}

impl<C: FieldEngine> ExpanderSingleVarChallenge<C> {
    pub fn empty() -> Self {
        Self {
            rz: vec![],
            r_simd: vec![],
            r_mpi: vec![],
        }
    }

    pub fn new(
        rz: Vec<C::ChallengeField>,
        r_simd: Vec<C::ChallengeField>,
        r_mpi: Vec<C::ChallengeField>,
    ) -> Self {
        Self { rz, r_simd, r_mpi }
    }

    pub fn local_xs(&self) -> Vec<C::ChallengeField> {
        let mut local_xs = vec![C::ChallengeField::ZERO; self.r_simd.len() + self.rz.len()];
        local_xs[..self.r_simd.len()].copy_from_slice(&self.r_simd);
        local_xs[self.r_simd.len()..].copy_from_slice(&self.rz);
        local_xs
    }

    pub fn global_xs(&self) -> Vec<C::ChallengeField> {
        let mut global_xs = vec![C::ChallengeField::ZERO; self.num_vars()];
        global_xs[..self.r_simd.len() + self.rz.len()].copy_from_slice(&self.local_xs());
        global_xs[self.r_simd.len() + self.rz.len()..].copy_from_slice(&self.r_mpi);
        global_xs
    }

    pub fn num_vars(&self) -> usize {
        self.rz.len() + self.r_simd.len() + self.r_mpi.len()
    }
}

impl<C: FieldEngine> ExpanderDualVarChallenge<C> {
    pub fn empty() -> Self {
        Self {
            rz_0: vec![],
            rz_1: None,
            r_simd: vec![],
            r_mpi: vec![],
        }
    }

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
            rz: self.rz_0.clone(),
            r_simd: self.r_simd.clone(),
            r_mpi: self.r_mpi.clone(),
        }
    }

    pub fn challenge_y(&self) -> ExpanderSingleVarChallenge<C> {
        assert!(self.rz_1.is_some());

        ExpanderSingleVarChallenge {
            rz: self.rz_1.clone().unwrap(),
            r_simd: self.r_simd.clone(),
            r_mpi: self.r_mpi.clone(),
        }
    }

    pub fn from_single_var_challenge(challenge: &ExpanderSingleVarChallenge<C>) -> Self {
        Self {
            rz_0: challenge.rz.clone(),
            rz_1: None,
            r_simd: challenge.r_simd.clone(),
            r_mpi: challenge.r_mpi.clone(),
        }
    }
}
