use arith::Field;

use super::Transcript;

#[derive(Debug, Clone, Default)]
pub struct ExpanderDualVarChallenge<F: Field> {
    /// random challenge for the x variable
    pub rz_0: Vec<F>,
    /// random challenge for the y variable
    pub rz_1: Option<Vec<F>>,
    /// random challenge to merge the SIMD circuit into a single one
    pub r_simd: Vec<F>,
    /// random challenge to merge the MPI circuit into a single one
    pub r_mpi: Vec<F>,
}

#[derive(Debug, Clone, Default)]
pub struct ExpanderSingleVarChallenge<F: Field> {
    /// random challenge for the main body of the circuit
    pub rz: Vec<F>,
    /// random challenge to merge the SIMD circuit into a single one
    pub r_simd: Vec<F>,
    /// random challenge to merge the MPI circuit into a single one
    pub r_mpi: Vec<F>,
}

impl<F: Field> ExpanderSingleVarChallenge<F> {
    #[inline]
    pub fn new(
        rz: Vec<F>,
        r_simd: Vec<F>,
        r_mpi: Vec<F>,
    ) -> Self {
        Self { rz, r_simd, r_mpi }
    }

    #[inline]
    pub fn local_xs(&self) -> Vec<F> {
        [self.r_simd.as_slice(), self.rz.as_slice()].concat()
    }

    #[inline]
    pub fn global_xs(&self) -> Vec<F> {
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
        transcript: &mut impl Transcript,
        num_circuit_var: usize,
        num_simd_var: usize,
        world_size: usize,
    ) -> Self {
        let rz = transcript.generate_field_elements::<F>(num_circuit_var);

        let r_simd = transcript.generate_field_elements::<F>(
            num_simd_var.trailing_zeros() as usize,
        );

        let r_mpi =
            transcript.generate_field_elements::<F>(world_size.trailing_zeros() as usize);

        Self { rz, r_simd, r_mpi }
    }
}

impl<F: Field> ExpanderDualVarChallenge<F> {
    #[inline]
    pub fn new(
        rz_0: Vec<F>,
        rz_1: Option<Vec<F>>,
        r_simd: Vec<F>,
        r_mpi: Vec<F>,
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
    pub fn challenge_y(&self) -> ExpanderSingleVarChallenge<F> {
        assert!(self.rz_1.is_some());

        ExpanderSingleVarChallenge {
            rz: self.rz_1.clone().unwrap(),
            r_simd: self.r_simd.clone(),
            r_mpi: self.r_mpi.clone(),
        }
    }

    #[inline]
    pub fn sample_from_transcript(
        transcript: &mut impl Transcript,
        num_circuit_var: usize,
        num_simd_var: usize,
        world_size: usize,
    ) -> Self {
        let rz_0 = transcript.generate_field_elements::<F>(num_circuit_var);

        let r_simd = transcript.generate_field_elements::<F>(
            num_simd_var.trailing_zeros() as usize,
        );

        let r_mpi =
            transcript.generate_field_elements::<F>(world_size.trailing_zeros() as usize);

        Self {
            rz_0,
            rz_1: None,
            r_simd,
            r_mpi,
        }
    }
}

impl<F: Field> From<ExpanderSingleVarChallenge<F>> for ExpanderDualVarChallenge<F> {
    fn from(challenge: ExpanderSingleVarChallenge<F>) -> Self {
        Self::from(&challenge)
    }
}

impl<F: Field> From<&ExpanderSingleVarChallenge<F>> for ExpanderDualVarChallenge<F> {
    fn from(challenge: &ExpanderSingleVarChallenge<F>) -> Self {
        Self {
            rz_0: challenge.rz.clone(),
            rz_1: None,
            r_simd: challenge.r_simd.clone(),
            r_mpi: challenge.r_mpi.clone(),
        }
    }
}
