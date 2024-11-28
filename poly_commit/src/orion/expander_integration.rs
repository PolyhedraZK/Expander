use std::marker::PhantomData;

use arith::{ExtensionField, Field, SimdField};
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use polynomials::MultiLinearPoly;
use transcript::Transcript;

use crate::{
    orion::*, traits::TensorCodeIOPPCS, ExpanderGKRChallenge, PCSForExpanderGKR,
    PolynomialCommitmentScheme, StructuredReferenceString,
};

impl StructuredReferenceString for OrionSRS {
    type PKey = OrionSRS;
    type VKey = OrionSRS;

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        (self.clone(), self.clone())
    }
}

pub struct OrionBaseFieldPCS<F, EvalF, ComPackF, OpenPackF, T>
where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    _marker_f: PhantomData<F>,
    _marker_eval_f: PhantomData<EvalF>,
    _marker_commit_f: PhantomData<ComPackF>,
    _marker_open_f: PhantomData<OpenPackF>,
    _marker_t: PhantomData<T>,
}

impl<F, EvalF, ComPackF, OpenPackF, T> PolynomialCommitmentScheme<EvalF, T>
    for OrionBaseFieldPCS<F, EvalF, ComPackF, OpenPackF, T>
where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    const NAME: &'static str = "OrionBaseFieldPCS";

    type Params = usize;
    type Poly = MultiLinearPoly<F>;
    type EvalPoint = Vec<EvalF>;
    type ScratchPad = OrionScratchPad<F, ComPackF>;

    type SRS = OrionSRS;
    type Commitment = OrionCommitment;
    type Opening = OrionProof<EvalF>;

    fn gen_srs_for_testing(params: &Self::Params, rng: impl rand::RngCore) -> Self::SRS {
        OrionSRS::from_random::<F>(*params, ORION_CODE_PARAMETER_INSTANCE, rng)
    }

    fn init_scratch_pad(_params: &Self::Params) -> Self::ScratchPad {
        OrionScratchPad::default()
    }

    fn commit(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        orion_commit_base_field(proving_key, poly, scratch_pad).unwrap()
    }

    fn open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        scratch_pad: &mut Self::ScratchPad,
        transcript: &mut T,
    ) -> (EvalF, Self::Opening) {
        orion_open_base_field::<F, EvalF, ComPackF, OpenPackF, T>(
            proving_key,
            poly,
            x,
            transcript,
            scratch_pad,
        )
    }

    fn verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: EvalF,
        opening: &Self::Opening,
        transcript: &mut T,
    ) -> bool {
        orion_verify_base_field::<F, EvalF, ComPackF, OpenPackF, T>(
            verifying_key,
            commitment,
            x,
            v,
            transcript,
            opening,
        )
    }
}

pub struct OrionSIMDFieldPCS<F, SimdF, EvalF, ComPackF, OpenPackF, T>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    _marker_f: PhantomData<F>,
    _marker_simd_f: PhantomData<SimdF>,
    _marker_eval_f: PhantomData<EvalF>,
    _marker_commit_f: PhantomData<ComPackF>,
    _marker_open_f: PhantomData<OpenPackF>,
    _marker_t: PhantomData<T>,
}

impl<F, SimdF, EvalF, ComPackF, OpenPackF, T> PolynomialCommitmentScheme<EvalF, T>
    for OrionSIMDFieldPCS<F, SimdF, EvalF, ComPackF, OpenPackF, T>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    const NAME: &'static str = "OrionSIMDFieldPCS";

    type Params = usize;
    type Poly = MultiLinearPoly<SimdF>;
    type EvalPoint = Vec<EvalF>;
    type ScratchPad = OrionScratchPad<F, ComPackF>;

    type SRS = OrionSRS;
    type Commitment = OrionCommitment;
    type Opening = OrionProof<EvalF>;

    // NOTE: here we say the number of variables is the sum of 2 following things:
    // - number of variables of the multilinear polynomial
    // - number of variables reside in the SIMD field - e.g., 3 vars for a SIMD 8 field
    fn gen_srs_for_testing(params: &Self::Params, rng: impl rand::RngCore) -> Self::SRS {
        OrionSRS::from_random::<F>(*params, ORION_CODE_PARAMETER_INSTANCE, rng)
    }

    fn init_scratch_pad(_params: &Self::Params) -> Self::ScratchPad {
        OrionScratchPad::default()
    }

    fn commit(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        orion_commit_simd_field(proving_key, poly, scratch_pad).unwrap()
    }

    fn open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        scratch_pad: &mut Self::ScratchPad,
        transcript: &mut T,
    ) -> (EvalF, Self::Opening) {
        let opening = orion_open_simd_field::<F, SimdF, EvalF, ComPackF, OpenPackF, T>(
            proving_key,
            poly,
            x,
            transcript,
            scratch_pad,
        );

        let real_num_vars = poly.get_num_vars() + SimdF::PACK_SIZE.ilog2() as usize;
        let num_vars_in_msg = {
            let (_, m) = <Self::SRS as TensorCodeIOPPCS>::evals_shape::<F>(real_num_vars);
            m + SimdF::PACK_SIZE.ilog2() as usize
        };

        let mut scratch = vec![EvalF::ZERO; 1 << num_vars_in_msg];
        let eval = MultiLinearPoly::evaluate_with_buffer(
            &opening.eval_row,
            &x[..num_vars_in_msg],
            &mut scratch,
        );
        drop(scratch);

        (eval, opening)
    }

    fn verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: EvalF,
        opening: &Self::Opening,
        transcript: &mut T,
    ) -> bool {
        orion_verify_simd_field::<F, SimdF, EvalF, ComPackF, OpenPackF, T>(
            verifying_key,
            commitment,
            x,
            v,
            transcript,
            opening,
        )
    }
}

// TODO ...
impl<C, ComPackF, OpenPackF, T> PCSForExpanderGKR<C, T>
    for OrionSIMDFieldPCS<
        C::CircuitField,
        C::SimdCircuitField,
        C::ChallengeField,
        ComPackF,
        OpenPackF,
        T,
    >
where
    C: GKRFieldConfig,
    ComPackF: SimdField<Scalar = C::CircuitField>,
    OpenPackF: SimdField<Scalar = C::CircuitField>,
    T: Transcript<C::ChallengeField>,
{
    const NAME: &'static str = "OrionSIMDPCSForExpanderGKR";

    type Params = usize;
    type ScratchPad = OrionScratchPad<C::CircuitField, ComPackF>;

    type Commitment = OrionCommitment;
    type Opening = OrionProof<C::ChallengeField>;
    type SRS = OrionSRS;

    #[allow(unused)]
    fn gen_params(n_input_vars: usize) -> Self::Params {
        todo!()
    }

    #[allow(unused)]
    fn gen_srs_for_testing(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        rng: impl rand::RngCore,
    ) -> Self::SRS {
        todo!()
    }

    fn init_scratch_pad(_params: &Self::Params, _mpi_config: &MPIConfig) -> Self::ScratchPad {
        Self::ScratchPad::default()
    }

    fn commit(
        _params: &Self::Params,
        mpi_config: &MPIConfig,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &MultiLinearPoly<C::SimdCircuitField>,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        let commitment = orion_commit_simd_field(proving_key, poly, scratch_pad).unwrap();
        if mpi_config.world_size == 1 {
            return commitment;
        }

        let local_buffer = vec![commitment.clone()];
        let mut buffer = match mpi_config.is_root() {
            true => vec![Self::Commitment::default(); mpi_config.world_size()],
            _ => Vec::new(),
        };
        mpi_config.gather_vec(&local_buffer, &mut buffer);

        let mut root = Self::Commitment::default();
        if mpi_config.is_root() {
            let final_tree_height = 1 + buffer.len().ilog2() as u32;
            let (internals, _) = tree::Tree::new_with_leaf_nodes(buffer.clone(), final_tree_height);
            root = internals[0];
        }
        mpi_config.root_broadcast_f(&mut root);
        root
    }

    fn open(
        _params: &Self::Params,
        mpi_config: &MPIConfig,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &MultiLinearPoly<C::SimdCircuitField>,
        eval_point: &ExpanderGKRChallenge<C>,
        transcript: &mut T, // add transcript here to allow interactive arguments
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Opening {
        let local_xs = eval_point.local_xs();
        let local_opening = orion_open_simd_field::<
            C::CircuitField,
            C::SimdCircuitField,
            C::ChallengeField,
            ComPackF,
            OpenPackF,
            T,
        >(proving_key, poly, &local_xs, transcript, scratch_pad);
        if mpi_config.world_size == 1 {
            return local_opening;
        }

        // TODO ... is x_mpi right of (earlier evaluated than) x_simd and x?

        todo!()
    }

    fn verify(
        _params: &Self::Params,
        mpi_config: &MPIConfig,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        eval_point: &ExpanderGKRChallenge<C>,
        v: C::ChallengeField,
        transcript: &mut T, // add transcript here to allow interactive arguments
        opening: &Self::Opening,
    ) -> bool {
        let local_xs = eval_point.local_xs();
        if mpi_config.world_size == 1 {
            return orion_verify_simd_field::<
                C::CircuitField,
                C::SimdCircuitField,
                C::ChallengeField,
                ComPackF,
                OpenPackF,
                T,
            >(verifying_key, commitment, &local_xs, v, transcript, opening);
        }

        todo!()
    }
}
