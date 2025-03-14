/// Raw commitment for multi-linear polynomials
use crate::{
    ExpanderGKRChallenge, PCSForExpanderGKR, PolynomialCommitmentScheme, StructuredReferenceString,
};
use arith::{ExtensionField, Field};
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use polynomials::{MultiLinearPoly, MultiLinearPolyExpander, MultilinearExtension};
use rand::RngCore;
use serdes::{ExpSerde, SerdeResult};
use transcript::Transcript;

#[derive(Clone, Debug, Default)]
pub struct RawCommitment<F: Field> {
    pub evals: Vec<F>,
}

impl<F: Field> ExpSerde for RawCommitment<F> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.evals.len().serialize_into(&mut writer)?;

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
pub struct RawMultiLinearScratchPad<F: Field> {
    pub eval_buffer: Vec<F>,
}

// Raw commitment for multi-linear polynomials
pub struct RawMultiLinearPCS {}

impl<F: ExtensionField, T: Transcript<F>> PolynomialCommitmentScheme<F, T> for RawMultiLinearPCS {
    const NAME: &'static str = "RawMultiLinear";

    type Params = usize;
    type ScratchPad = ();

    type Poly = MultiLinearPoly<F>;

    type EvalPoint = Vec<F>;

    type SRS = ();
    type Commitment = RawCommitment<F>;

    type Opening = ();

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
        assert!(poly.coeffs.len() == 1 << params);
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
        assert!(x.len() == *params);
        (MultiLinearPoly::<F>::evaluate_jolt(poly, x), ())
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
        assert!(x.len() == *params);
        MultiLinearPoly::<F>::evaluate_with_buffer(
            &commitment.evals,
            x,
            &mut vec![F::ZERO; commitment.evals.len()],
        ) == v
    }
}

// =================================================================================================

pub struct RawExpanderGKR<C: GKRFieldConfig, T: Transcript<C::ChallengeField>> {
    _phantom: std::marker::PhantomData<(C, T)>,
}

impl<C: GKRFieldConfig, T: Transcript<C::ChallengeField>> PCSForExpanderGKR<C, T>
    for RawExpanderGKR<C, T>
{
    const NAME: &'static str = "RawExpanderGKR";

    type Params = usize;

    // type Poly = MultiLinearPoly<C::SimdCircuitField>;

    // type EvalPoint = (
    //     Vec<C::ChallengeField>, // x
    //     Vec<C::ChallengeField>, // x_simd
    //     Vec<C::ChallengeField>, // x_mpi
    // );

    type ScratchPad = ();

    type SRS = ();

    type Commitment = RawCommitment<C::SimdCircuitField>;

    type Opening = ();

    fn gen_srs_for_testing(
        _params: &Self::Params,
        _mpi_config: &MPIConfig,
        _rng: impl RngCore,
    ) -> Self::SRS {
    }

    fn gen_params(n_input_vars: usize) -> Self::Params {
        n_input_vars
    }

    fn init_scratch_pad(_params: &Self::Params, _mpi_config: &MPIConfig) -> Self::ScratchPad {}

    fn commit(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        _proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl MultilinearExtension<C::SimdCircuitField>,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        assert!(poly.num_vars() == *params);

        if mpi_config.is_single_process() {
            return Self::Commitment {
                evals: poly.hypercube_basis(),
            };
        }

        let mut buffer = if mpi_config.is_root() {
            vec![C::SimdCircuitField::zero(); poly.hypercube_size() * mpi_config.world_size()]
        } else {
            vec![]
        };

        mpi_config.gather_vec(poly.hypercube_basis_ref(), &mut buffer);

        Self::Commitment { evals: buffer }
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
    }

    fn verify(
        _params: &Self::Params,
        _verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &ExpanderGKRChallenge<C>,
        v: C::ChallengeField,
        _transcript: &mut T,
        _opening: &Self::Opening,
    ) -> bool {
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
