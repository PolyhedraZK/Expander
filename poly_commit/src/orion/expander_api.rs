use arith::{Field, SimdField};
use gkr_engine::{
    ExpanderPCS, ExpanderSingleVarChallenge, FieldEngine, MPIEngine, PolynomialCommitmentType,
    StructuredReferenceString, Transcript,
};
use polynomials::MultilinearExtension;

use crate::orion::{
    simd_field_impl::{orion_commit_simd_field, orion_open_simd_field},
    simd_field_mpi_impl::{orion_mpi_commit_simd_field, orion_mpi_open_simd_field},
    verify::orion_verify,
    OrionCommitment, OrionProof, OrionSIMDFieldPCS, OrionSRS, OrionScratchPad,
    ORION_CODE_PARAMETER_INSTANCE,
};

impl<C, ComPackF> ExpanderPCS<C>
    for OrionSIMDFieldPCS<C::CircuitField, C::SimdCircuitField, C::ChallengeField, ComPackF>
where
    C: FieldEngine,
    ComPackF: SimdField<Scalar = C::CircuitField>,
{
    const NAME: &'static str = "OrionPCSForExpanderGKR";

    const PCS_TYPE: PolynomialCommitmentType = PolynomialCommitmentType::Orion;

    type Params = usize;
    type ScratchPad = OrionScratchPad;

    type Commitment = OrionCommitment;
    type Opening = OrionProof<C::ChallengeField>;
    type SRS = OrionSRS;

    /// NOTE(HS): this is the number of variables for local polynomial w.r.t. SIMD field elements.
    fn gen_params(n_input_vars: usize) -> Self::Params {
        n_input_vars
    }

    fn gen_srs_for_testing(
        params: &Self::Params,
        mpi_engine: &impl MPIEngine,
        rng: impl rand::RngCore,
    ) -> (Self::SRS, usize) {
        let num_vars_each_core = *params + C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
        let (srs, calibrated_num_vars_each_core) = OrionSRS::from_random(
            mpi_engine.world_size(),
            num_vars_each_core,
            C::CircuitField::FIELD_SIZE,
            ComPackF::PACK_SIZE,
            ORION_CODE_PARAMETER_INSTANCE,
            rng,
        );
        let calibrated_num_local_simd_vars =
            calibrated_num_vars_each_core - C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
        (srs, calibrated_num_local_simd_vars)
    }

    fn init_scratch_pad(_params: &Self::Params, _mpi_engine: &impl MPIEngine) -> Self::ScratchPad {
        Self::ScratchPad::default()
    }

    fn commit(
        params: &Self::Params,
        mpi_engine: &impl MPIEngine,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl MultilinearExtension<C::SimdCircuitField>,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Option<Self::Commitment> {
        let num_vars_each_core = *params + C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
        assert_eq!(num_vars_each_core, proving_key.num_vars);

        if mpi_engine.is_single_process() {
            return orion_commit_simd_field::<_, C::SimdCircuitField, ComPackF>(
                proving_key,
                poly,
                scratch_pad,
            )
            .ok();
        }

        orion_mpi_commit_simd_field::<_, C::SimdCircuitField, ComPackF>(
            mpi_engine,
            proving_key,
            poly,
            scratch_pad,
        )
        .ok()
    }

    fn open(
        params: &Self::Params,
        mpi_engine: &impl MPIEngine,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl MultilinearExtension<C::SimdCircuitField>,
        eval_point: &ExpanderSingleVarChallenge<C>,
        transcript: &mut impl Transcript<C::ChallengeField>,
        scratch_pad: &Self::ScratchPad,
    ) -> Option<Self::Opening> {
        let num_vars_each_core = *params + C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
        assert_eq!(num_vars_each_core, proving_key.num_vars);

        if mpi_engine.is_single_process() {
            let (_, opening) = orion_open_simd_field::<_, C::SimdCircuitField, _, ComPackF>(
                proving_key,
                poly,
                &eval_point.local_xs(),
                transcript,
                scratch_pad,
            );
            return opening.into();
        }

        orion_mpi_open_simd_field::<_, C::SimdCircuitField, _, ComPackF>(
            mpi_engine,
            proving_key,
            poly,
            &eval_point.local_xs(),
            &eval_point.r_mpi,
            transcript,
            scratch_pad,
        )
    }

    fn verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        eval_point: &ExpanderSingleVarChallenge<C>,
        eval: C::ChallengeField,
        transcript: &mut impl Transcript<C::ChallengeField>,
        opening: &Self::Opening,
    ) -> bool {
        orion_verify::<_, C::SimdCircuitField, _, ComPackF>(
            verifying_key,
            commitment,
            &eval_point.local_xs(),
            &eval_point.r_mpi,
            eval,
            transcript,
            opening,
        )
    }
}

pub type OrionPCSForGKR<C, ComPack> = OrionSIMDFieldPCS<
    <C as FieldEngine>::CircuitField,
    <C as FieldEngine>::SimdCircuitField,
    <C as FieldEngine>::ChallengeField,
    ComPack,
>;
