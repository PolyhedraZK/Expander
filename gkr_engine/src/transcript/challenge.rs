use arith::SimdField;

use crate::FieldEngine;

use super::Transcript;

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
    #[inline]
    pub fn new(
        rz: Vec<C::ChallengeField>,
        r_simd: Vec<C::ChallengeField>,
        r_mpi: Vec<C::ChallengeField>,
    ) -> Self {
        Self { rz, r_simd, r_mpi }
    }

    #[inline]
    pub fn local_xs(&self) -> Vec<C::ChallengeField> {
        [self.r_simd.as_slice(), self.rz.as_slice()].concat()
    }

    #[inline]
    pub fn global_xs(&self) -> Vec<C::ChallengeField> {
        [
            self.r_simd.as_slice(),
            self.rz.as_slice(),
            self.r_mpi.as_slice(),
        ]
        .concat()
    }

    #[inline]
    pub fn num_vars(&self) -> usize {
        self.rz.len() + self.r_simd.len() + self.r_mpi.len()
    }

    #[inline]
    pub fn sample_from_transcript(
        transcript: &mut impl Transcript<C::ChallengeField>,
        num_circuit_var: usize,
        world_size: usize,
    ) -> Self {
        let rz = transcript.generate_challenge_field_elements(num_circuit_var);

        let r_simd = transcript.generate_challenge_field_elements(
            <C::SimdCircuitField as SimdField>::PACK_SIZE.trailing_zeros() as usize,
        );

        let r_mpi =
            transcript.generate_challenge_field_elements(world_size.trailing_zeros() as usize);

        Self { rz, r_simd, r_mpi }
    }
}

impl<C: FieldEngine> ExpanderDualVarChallenge<C> {
    #[inline]
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

    #[inline]
    pub fn challenge_x(&self) -> ExpanderSingleVarChallenge<C> {
        ExpanderSingleVarChallenge {
            rz: self.rz_0.clone(),
            r_simd: self.r_simd.clone(),
            r_mpi: self.r_mpi.clone(),
        }
    }

    #[inline]
    pub fn challenge_y(&self) -> ExpanderSingleVarChallenge<C> {
        assert!(self.rz_1.is_some());

        ExpanderSingleVarChallenge {
            rz: self.rz_1.clone().unwrap(),
            r_simd: self.r_simd.clone(),
            r_mpi: self.r_mpi.clone(),
        }
    }

    #[inline]
    pub fn sample_from_transcript(
        transcript: &mut impl Transcript<C::ChallengeField>,
        num_circuit_var: usize,
        world_size: usize,
    ) -> Self {
        let rz_0 = transcript.generate_challenge_field_elements(num_circuit_var);

        let r_simd = transcript.generate_challenge_field_elements(
            <C::SimdCircuitField as SimdField>::PACK_SIZE.trailing_zeros() as usize,
        );

        let r_mpi =
            transcript.generate_challenge_field_elements(world_size.trailing_zeros() as usize);

        Self {
            rz_0,
            rz_1: None,
            r_simd,
            r_mpi,
        }
    }
}

impl<C: FieldEngine> From<ExpanderSingleVarChallenge<C>> for ExpanderDualVarChallenge<C> {
    fn from(challenge: ExpanderSingleVarChallenge<C>) -> Self {
        Self::from(&challenge)
    }
}

impl<C: FieldEngine> From<&ExpanderSingleVarChallenge<C>> for ExpanderDualVarChallenge<C> {
    fn from(challenge: &ExpanderSingleVarChallenge<C>) -> Self {
        Self {
            rz_0: challenge.rz.clone(),
            rz_1: None,
            r_simd: challenge.r_simd.clone(),
            r_mpi: challenge.r_mpi.clone(),
        }
    }
}

impl<C: FieldEngine> From<ExpanderDualVarChallenge<C>> for ExpanderSingleVarChallenge<C> {
    fn from(value: ExpanderDualVarChallenge<C>) -> Self {
        Self::from(&value)
    }
}

impl<C: FieldEngine> From<&ExpanderDualVarChallenge<C>> for ExpanderSingleVarChallenge<C> {
    fn from(value: &ExpanderDualVarChallenge<C>) -> Self {
        Self {
            rz: value.rz_0.clone(),
            r_simd: value.r_simd.clone(),
            r_mpi: value.r_mpi.clone(),
        }
    }
}
