use std::io::{Read, Write};

use arith::SimdField;
use serdes::{ExpSerde, SerdeResult};

use crate::FieldEngine;

use super::Transcript;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExpanderDualVarChallenge<F: FieldEngine> {
    /// random challenge for the x variable
    pub rz_0: Vec<F::ChallengeField>,
    /// random challenge for the y variable
    pub rz_1: Option<Vec<F::ChallengeField>>,
    /// random challenge to merge the SIMD circuit into a single one
    pub r_simd: Vec<F::ChallengeField>,
    /// random challenge to merge the MPI circuit into a single one
    pub r_mpi: Vec<F::ChallengeField>,
}

#[derive(Debug, Clone, Default)]
pub struct ExpanderSingleVarChallenge<F: FieldEngine> {
    /// random challenge for the main body of the circuit
    pub rz: Vec<F::ChallengeField>,
    /// random challenge to merge the SIMD circuit into a single one
    pub r_simd: Vec<F::ChallengeField>,
    /// random challenge to merge the MPI circuit into a single one
    pub r_mpi: Vec<F::ChallengeField>,
}

impl<F: FieldEngine> ExpanderSingleVarChallenge<F> {
    #[inline]
    pub fn new(
        rz: Vec<F::ChallengeField>,
        r_simd: Vec<F::ChallengeField>,
        r_mpi: Vec<F::ChallengeField>,
    ) -> Self {
        Self { rz, r_simd, r_mpi }
    }

    #[inline]
    pub fn local_xs(&self) -> Vec<F::ChallengeField> {
        [self.r_simd.as_slice(), self.rz.as_slice()].concat()
    }

    #[inline]
    pub fn global_xs(&self) -> Vec<F::ChallengeField> {
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
        transcript: &mut impl Transcript<F::ChallengeField>,
        num_circuit_var: usize,
        world_size: usize,
    ) -> Self {
        let rz = transcript.generate_challenge_field_elements(num_circuit_var);

        let r_simd = transcript.generate_challenge_field_elements(
            <F::SimdCircuitField as SimdField>::PACK_SIZE.trailing_zeros() as usize,
        );

        let r_mpi =
            transcript.generate_challenge_field_elements(world_size.trailing_zeros() as usize);

        Self { rz, r_simd, r_mpi }
    }
}

impl<F: FieldEngine> ExpanderDualVarChallenge<F> {
    #[inline]
    pub fn new(
        rz_0: Vec<F::ChallengeField>,
        rz_1: Option<Vec<F::ChallengeField>>,
        r_simd: Vec<F::ChallengeField>,
        r_mpi: Vec<F::ChallengeField>,
    ) -> Self {
        Self {
            rz_0,
            rz_1,
            r_simd,
            r_mpi,
        }
    }

    #[inline]
    pub fn challenge_x(&self) -> ExpanderSingleVarChallenge<F> {
        ExpanderSingleVarChallenge {
            rz: self.rz_0.clone(),
            r_simd: self.r_simd.clone(),
            r_mpi: self.r_mpi.clone(),
        }
    }

    #[inline]
    pub fn challenge_y(&self) -> Option<ExpanderSingleVarChallenge<F>> {
        self.rz_1.as_ref().map(|rz_1| ExpanderSingleVarChallenge {
            rz: rz_1.clone(),
            r_simd: self.r_simd.clone(),
            r_mpi: self.r_mpi.clone(),
        })
    }

    #[inline]
    pub fn sample_from_transcript(
        transcript: &mut impl Transcript<F::ChallengeField>,
        num_circuit_var: usize,
        world_size: usize,
    ) -> Self {
        let rz_0 = transcript.generate_challenge_field_elements(num_circuit_var);

        let r_simd = transcript.generate_challenge_field_elements(
            <F::SimdCircuitField as SimdField>::PACK_SIZE.trailing_zeros() as usize,
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

impl<F: FieldEngine> From<ExpanderSingleVarChallenge<F>> for ExpanderDualVarChallenge<F> {
    fn from(challenge: ExpanderSingleVarChallenge<F>) -> Self {
        Self::from(&challenge)
    }
}

impl<F: FieldEngine> From<&ExpanderSingleVarChallenge<F>> for ExpanderDualVarChallenge<F> {
    fn from(challenge: &ExpanderSingleVarChallenge<F>) -> Self {
        Self {
            rz_0: challenge.rz.clone(),
            rz_1: None,
            r_simd: challenge.r_simd.clone(),
            r_mpi: challenge.r_mpi.clone(),
        }
    }
}

impl<F: FieldEngine> ExpSerde for ExpanderDualVarChallenge<F> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.rz_0.serialize_into(&mut writer)?;
        self.rz_1.serialize_into(&mut writer)?;
        self.r_simd.serialize_into(&mut writer)?;
        self.r_mpi.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let rz_0 = Vec::<F::ChallengeField>::deserialize_from(&mut reader)?;
        let rz_1 = Option::<Vec<F::ChallengeField>>::deserialize_from(&mut reader)?;
        let r_simd = Vec::<F::ChallengeField>::deserialize_from(&mut reader)?;
        let r_mpi = Vec::<F::ChallengeField>::deserialize_from(&mut reader)?;

        Ok(Self {
            rz_0,
            rz_1,
            r_simd,
            r_mpi,
        })
    }
}

impl<F: FieldEngine> ExpSerde for ExpanderSingleVarChallenge<F> {
    const SERIALIZED_SIZE: usize = unimplemented!();
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.rz.serialize_into(&mut writer)?;
        self.r_simd.serialize_into(&mut writer)?;
        self.r_mpi.serialize_into(&mut writer)?;
        Ok(())
    }
    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let rz = Vec::<F::ChallengeField>::deserialize_from(&mut reader)?;
        let r_simd = Vec::<F::ChallengeField>::deserialize_from(&mut reader)?;
        let r_mpi = Vec::<F::ChallengeField>::deserialize_from(&mut reader)?;

        Ok(Self { rz, r_simd, r_mpi })
    }
}
