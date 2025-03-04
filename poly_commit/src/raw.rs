/// Raw commitment for multi-linear polynomials
use crate::{
    ExpanderGKRChallenge, PCSEmptyType, PCSForExpanderGKR, PolynomialCommitmentScheme,
    StructuredReferenceString,
};
use arith::{ExtensionField, Field};
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use polynomials::{MultiLinearPoly, MultiLinearPolyExpander, MultilinearExtension};
use rand::RngCore;
use serdes::{ArithSerde, ExpSerde, SerdeResult};
use transcript::Transcript;

#[derive(Clone, Debug, Default)]
pub struct RawCommitment<F: Field> {
    pub evals: Vec<F>,
}

impl<F: Field> ExpSerde for RawCommitment<F> {
    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        let len = self.evals.len();
        writer.write_all(len.to_le_bytes().as_ref())?;

        self.evals
            .iter()
            .try_for_each(|v| v.serialize_into(&mut writer))?;

        Ok(())
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> SerdeResult<Self> {
        let mut v = Self::default();

        let len = usize::deserialize_from(&mut reader)?;

        for _ in 0..len {
            v.evals.push(F::deserialize_from(&mut reader)?);
        }
        Ok(v)
    }
}

#[derive(Clone, Debug, Default)]
pub struct RawMultiLinearParams {
    pub n_vars: usize,
}

#[derive(Clone, Debug, Default)]
pub struct RawMultiLinearScratchPad<F: Field> {
    pub eval_buffer: Vec<F>,
}

// Raw commitment for multi-linear polynomials
pub struct RawMultiLinearPCS {}

impl<F: ExtensionField, T: Transcript<F>> PolynomialCommitmentScheme<F, T> for RawMultiLinearPCS {
    const NAME: &'static str = "RawMultiLinear";

    type Params = RawMultiLinearParams;
    type ScratchPad = ();

    type Poly = MultiLinearPoly<F>;

    type EvalPoint = Vec<F>;

    type SRS = PCSEmptyType;
    type Commitment = RawCommitment<F>;

    type Opening = PCSEmptyType;

    fn gen_srs_for_testing(_params: &Self::Params, _rng: impl RngCore) -> Self::SRS {
        Self::SRS::default()
    }

    fn init_scratch_pad(_params: &Self::Params) -> Self::ScratchPad {}

    fn commit(
        params: &Self::Params,
        _proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        assert!(poly.coeffs.len() == 1 << params.n_vars);
        Self::Commitment {
            evals: poly.coeffs.clone(),
        }
    }

    fn open(
        params: &Self::Params,
        _proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        _scratch_pad: &Self::ScratchPad,
        _transcript: &mut T,
    ) -> (F, Self::Opening) {
        assert!(x.len() == params.n_vars);
        (
            MultiLinearPoly::<F>::evaluate_jolt(poly, x),
            Self::Opening::default(),
        )
    }

    fn verify(
        params: &Self::Params,
        _verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: F,
        _opening: &Self::Opening,
        _transcript: &mut T,
    ) -> bool {
        assert!(x.len() == params.n_vars);
        MultiLinearPoly::<F>::evaluate_with_buffer(
            &commitment.evals,
            x,
            &mut vec![F::ZERO; commitment.evals.len()],
        ) == v
    }
}

// =================================================================================================

#[derive(Clone, Debug, Default)]
pub struct RawExpanderGKRParams {
    pub n_local_vars: usize,
}

pub struct RawExpanderGKR<C: GKRFieldConfig, T: Transcript<C::ChallengeField>> {
    _phantom: std::marker::PhantomData<(C, T)>,
}

impl<C: GKRFieldConfig, T: Transcript<C::ChallengeField>> PCSForExpanderGKR<C, T>
    for RawExpanderGKR<C, T>
{
    const NAME: &'static str = "RawExpanderGKR";

    type Params = RawExpanderGKRParams;

    // type Poly = MultiLinearPoly<C::SimdCircuitField>;

    // type EvalPoint = (
    //     Vec<C::ChallengeField>, // x
    //     Vec<C::ChallengeField>, // x_simd
    //     Vec<C::ChallengeField>, // x_mpi
    // );

    type ScratchPad = ();

    type SRS = PCSEmptyType;

    type Commitment = RawCommitment<C::SimdCircuitField>;

    type Opening = PCSEmptyType;

    fn gen_srs_for_testing(
        _params: &Self::Params,
        _mpi_config: &MPIConfig,
        _rng: impl RngCore,
    ) -> Self::SRS {
        Self::SRS::default()
    }

    fn gen_params(n_input_vars: usize) -> Self::Params {
        RawExpanderGKRParams {
            n_local_vars: n_input_vars,
        }
    }

    fn init_scratch_pad(_params: &Self::Params, _mpi_config: &MPIConfig) -> Self::ScratchPad {}

    fn commit(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        _proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl MultilinearExtension<C::SimdCircuitField>,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        assert!(poly.num_vars() == params.n_local_vars);
        let evals = if mpi_config.world_size() == 1 {
            poly.hypercube_basis()
        } else {
            let mut buffer = if mpi_config.is_root() {
                vec![C::SimdCircuitField::zero(); poly.hypercube_size() * mpi_config.world_size()]
            } else {
                vec![]
            };

            mpi_config.gather_vec(poly.hypercube_basis_ref(), &mut buffer);
            buffer
        };
        Self::Commitment { evals }
    }

    fn open(
        _params: &Self::Params,
        _mpi_config: &MPIConfig,
        _proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        _poly: &impl MultilinearExtension<C::SimdCircuitField>,
        _x: &ExpanderGKRChallenge<C>,
        _transcript: &mut T,
        _scratch_pad: &Self::ScratchPad,
    ) -> Self::Opening {
        // For GKR, we don't need the evaluation result
        Self::Opening::default()
    }

    fn verify(
        _params: &Self::Params,
        mpi_config: &MPIConfig,
        _verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &ExpanderGKRChallenge<C>,
        v: C::ChallengeField,
        _transcript: &mut T,
        _opening: &Self::Opening,
    ) -> bool {
        assert!(mpi_config.is_root()); // Only the root will verify
        let ExpanderGKRChallenge::<C> { x, x_simd, x_mpi } = x;
        let v_target =
            MultiLinearPolyExpander::<C>::single_core_eval_circuit_vals_at_expander_challenge(
                &commitment.evals,
                x,
                x_simd,
                x_mpi,
            );
        v == v_target
    }
}
