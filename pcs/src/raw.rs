/// Raw commitment for multi-linear polynomials
use crate::{GKRChallenge, PCSEmptyType, PCSForGKR, PCS, SRS};
use arith::{Field, FieldSerde, SimdField};
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use polynomials::MultiLinearPoly;
use rand::RngCore;

#[derive(Clone, Debug, Default)]
pub struct RawMLParams {
    pub n_vars: usize,
}

#[derive(Clone, Debug, Default)]
pub struct RawMLScratchPad<F: Field + FieldSerde> {
    pub eval_buffer: Vec<F>,
}

// Raw commitment for multi-linear polynomials
pub struct RawML {}

impl<F: Field + FieldSerde> PCS<F> for RawML {
    const NAME: &'static str = "RawML";

    type Params = RawMLParams;
    type ScratchPad = RawMLScratchPad<F>;

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
        _proving_key: &<Self::SRS as SRS>::PKey,
        poly: &Self::Poly,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        assert!(poly.coeffs.len() == 1 << params.n_vars);
        poly.coeffs.clone()
    }

    fn open(
        params: &Self::Params,
        _proving_key: &<Self::SRS as SRS>::PKey,
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
        _verifying_key: &<Self::SRS as SRS>::VKey,
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
pub struct RawMLGKRParams {
    pub n_local_vars: usize,
    pub mpi_config: MPIConfig,
}

#[derive(Clone, Debug, Default)]
pub struct RawMLGKRScratchPad {}

pub struct RawMLGKR<C: GKRFieldConfig> {
    _phantom: std::marker::PhantomData<C>,
}

impl<C: GKRFieldConfig> PCSForGKR<C> for RawMLGKR<C> {
    const NAME: &'static str = "RawMLGKR";

    type Params = RawMLGKRParams;

    // type Poly = MultiLinearPoly<C::SimdCircuitField>;

    // type EvalPoint = (
    //     Vec<C::ChallengeField>, // x
    //     Vec<C::ChallengeField>, // x_simd
    //     Vec<C::ChallengeField>, // x_mpi
    // );

    type ScratchPad = RawMLGKRScratchPad;

    type SRS = PCSEmptyType;

    type Commitment = Vec<C::SimdCircuitField>;

    type Opening = PCSEmptyType;

    fn gen_srs_for_testing(_params: &Self::Params, _rng: impl RngCore) -> Self::SRS {
        Self::SRS::default()
    }

    fn init_scratch_pad(_params: &Self::Params) -> Self::ScratchPad {
        RawMLGKRScratchPad {}
    }

    fn commit(
        params: &Self::Params,
        _proving_key: &<Self::SRS as SRS>::PKey,
        poly: &MultiLinearPoly<C::SimdCircuitField>,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        assert!(poly.coeffs.len() == 1 << params.n_local_vars);
        let mpi_config = &params.mpi_config;
        if mpi_config.world_size() == 1 {
            poly.coeffs.clone()
        } else {
            let mut buffer = if mpi_config.is_root() {
                vec![C::SimdCircuitField::zero(); poly.coeffs.len() * mpi_config.world_size()]
            } else {
                vec![]
            };

            mpi_config.gather_vec(&poly.coeffs, &mut buffer);
            buffer
        }
    }

    fn open(
        _params: &Self::Params,
        _proving_key: &<Self::SRS as SRS>::PKey,
        _poly: &MultiLinearPoly<C::SimdCircuitField>,
        _x: &GKRChallenge<C>,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Opening {
        // For GKR, we don't need the evaluation result
        Self::Opening::default()
    }

    fn verify(
        _params: &Self::Params,
        _verifying_key: &<Self::SRS as SRS>::VKey,
        commitment: &Self::Commitment,
        x: &GKRChallenge<C>,
        v: C::ChallengeField,
        _opening: &Self::Opening,
    ) -> bool {
        let GKRChallenge::<C> { x, x_simd, x_mpi } = x;
        Self::eval(commitment, x, x_simd, x_mpi) == v
    }
}

impl<C: GKRFieldConfig> RawMLGKR<C> {
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
