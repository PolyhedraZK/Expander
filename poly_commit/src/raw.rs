/// Raw commitment for multi-linear polynomials
use crate::{
    ExpanderGKRChallenge, PCSEmptyType, PCSForExpanderGKR, PolynomialCommitmentScheme,
    StructuredReferenceString,
};
use arith::{Field, SimdField};
use communicator::{ExpanderComm, MPICommunicator};
use gkr_field_config::GKRFieldConfig;
use polynomials::MultiLinearPoly;
use rand::RngCore;
use transcript::Transcript;

#[derive(Clone, Debug, Default)]
pub struct RawMultiLinearParams {
    pub n_vars: usize,
}

#[derive(Clone, Debug, Default)]
pub struct RawMultiLinearScratchPad<F: Field> {
    pub eval_buffer: Vec<F>,
}

// Raw commitment for multi-linear polynomials
pub struct RawMultiLinear {}

impl<F: Field> PolynomialCommitmentScheme<F> for RawMultiLinear {
    const NAME: &'static str = "RawMultiLinear";

    type Params = RawMultiLinearParams;
    type ScratchPad = RawMultiLinearScratchPad<F>;

    type Poly = MultiLinearPoly<F>;

    type EvalPoint = Vec<F>;

    type SRS = PCSEmptyType;
    type Commitment = Vec<F>;

    type Opening = PCSEmptyType;

    fn gen_srs_for_testing(_params: &Self::Params, _rng: impl RngCore) -> Self::SRS {
        Self::SRS::default()
    }

    fn init_scratch_pad(params: &Self::Params) -> Self::ScratchPad {
        Self::ScratchPad {
            eval_buffer: vec![F::ZERO; 1 << params.n_vars],
        }
    }

    fn commit(
        params: &Self::Params,
        _proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        assert!(poly.coeffs.len() == 1 << params.n_vars);
        poly.coeffs.clone()
    }

    fn open(
        params: &Self::Params,
        _proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        scratch_pad: &mut Self::ScratchPad,
    ) -> (F, Self::Opening) {
        assert!(x.len() == params.n_vars);
        (
            MultiLinearPoly::<F>::evaluate_with_buffer(
                &poly.coeffs,
                x,
                &mut scratch_pad.eval_buffer,
            ),
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
    ) -> bool {
        assert!(x.len() == params.n_vars);
        MultiLinearPoly::<F>::evaluate_with_buffer(
            commitment,
            x,
            &mut vec![F::ZERO; commitment.len()],
        ) == v
    }
}

// =================================================================================================

#[derive(Clone, Debug, Default)]
pub struct RawExpanderGKRParams {
    pub n_local_vars: usize,
}

#[derive(Clone, Debug, Default)]
pub struct RawExpanderGKRScratchPad {}

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

    type ScratchPad = RawExpanderGKRScratchPad;

    type SRS = PCSEmptyType;

    type Commitment = Vec<C::SimdCircuitField>;

    type Opening = PCSEmptyType;

    fn gen_srs_for_testing(
        _params: &Self::Params,
        _mpi_comm: &MPICommunicator,
        _rng: impl RngCore,
    ) -> Self::SRS {
        Self::SRS::default()
    }

    fn gen_params(n_input_vars: usize) -> Self::Params {
        RawExpanderGKRParams {
            n_local_vars: n_input_vars,
        }
    }

    fn init_scratch_pad(_params: &Self::Params, _mpi_comm: &MPICommunicator) -> Self::ScratchPad {
        RawExpanderGKRScratchPad {}
    }

    fn commit(
        params: &Self::Params,
        mpi_comm: &MPICommunicator,
        _proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &MultiLinearPoly<C::SimdCircuitField>,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        assert!(poly.coeffs.len() == 1 << params.n_local_vars);
        if mpi_comm.world_size() == 1 {
            poly.coeffs.clone()
        } else {
            let mut buffer = if mpi_comm.is_root() {
                vec![C::SimdCircuitField::zero(); poly.coeffs.len() * mpi_comm.world_size()]
            } else {
                vec![]
            };

            mpi_comm.gather_vec(&poly.coeffs, &mut buffer);
            buffer
        }
    }

    fn open(
        _params: &Self::Params,
        _mpi_comm: &MPICommunicator,
        _proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        _poly: &MultiLinearPoly<C::SimdCircuitField>,
        _x: &ExpanderGKRChallenge<C>,
        _transcript: &mut T,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Opening {
        // For GKR, we don't need the evaluation result
        Self::Opening::default()
    }

    fn verify(
        _params: &Self::Params,
        mpi_comm: &MPICommunicator,
        _verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &ExpanderGKRChallenge<C>,
        v: C::ChallengeField,
        _transcript: &mut T,
        _opening: &Self::Opening,
    ) -> bool {
        assert!(mpi_comm.is_root()); // Only the root will verify
        let ExpanderGKRChallenge::<C> { x, x_simd, x_mpi } = x;
        Self::eval(commitment, x, x_simd, x_mpi) == v
    }
}

impl<C: GKRFieldConfig, T: Transcript<C::ChallengeField>> RawExpanderGKR<C, T> {
    pub fn eval_local(
        vals: &[C::SimdCircuitField],
        x: &[C::ChallengeField],
        x_simd: &[C::ChallengeField],
    ) -> C::ChallengeField {
        let mut scratch = vec![C::Field::default(); vals.len()];
        let y_simd = C::eval_circuit_vals_at_challenge(vals, x, &mut scratch);
        let y_simd_unpacked = y_simd.unpack();
        let mut scratch = vec![C::ChallengeField::default(); y_simd_unpacked.len()];
        MultiLinearPoly::evaluate_with_buffer(&y_simd_unpacked, x_simd, &mut scratch)
    }

    pub fn eval(
        vals: &[C::SimdCircuitField],
        x: &[C::ChallengeField],
        x_simd: &[C::ChallengeField],
        x_mpi: &[C::ChallengeField],
    ) -> C::ChallengeField {
        let local_poly_size = vals.len() >> x_mpi.len();
        let local_evals = vals
            .chunks(local_poly_size)
            .map(|local_vals| Self::eval_local(local_vals, x, x_simd))
            .collect::<Vec<C::ChallengeField>>();

        let mut scratch = vec![C::ChallengeField::default(); local_evals.len()];
        MultiLinearPoly::evaluate_with_buffer(&local_evals, x_mpi, &mut scratch)
    }
}
