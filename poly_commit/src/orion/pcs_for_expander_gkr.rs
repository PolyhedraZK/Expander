use arith::SimdField;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use polynomials::MultilinearExtension;
use transcript::Transcript;

use crate::{
    orion::{
        simd_field_impl::{orion_commit_simd_field, orion_open_simd_field},
        simd_field_mpi_impl::{orion_mpi_commit_simd_field, orion_mpi_open_simd_field},
        verify::orion_verify,
        OrionCommitment, OrionProof, OrionSIMDFieldPCS, OrionSRS, OrionScratchPad,
        ORION_CODE_PARAMETER_INSTANCE,
    },
    traits::TensorCodeIOPPCS,
    ExpanderGKRChallenge, PCSForExpanderGKR, StructuredReferenceString,
};

impl<C, ComPackF, T> PCSForExpanderGKR<C, T>
    for OrionSIMDFieldPCS<C::CircuitField, C::SimdCircuitField, C::ChallengeField, ComPackF, T>
where
    C: GKRFieldConfig,
    ComPackF: SimdField<Scalar = C::CircuitField>,
    T: Transcript<C::ChallengeField>,
{
    const NAME: &'static str = "OrionPCSForExpanderGKR";

    type Params = usize;
    type ScratchPad = OrionScratchPad<C::CircuitField, ComPackF>;

    type Commitment = OrionCommitment;
    type Opening = OrionProof<C::ChallengeField>;
    type SRS = OrionSRS;

    fn minimum_num_vars(world_size: usize) -> usize {
        let circuit_field_elems_per_leaf = tree::leaf_adic::<C::CircuitField>();
        let leaves_per_mt_opening = Self::SRS::LEAVES_IN_RANGE_OPENING * world_size;

        let circuit_field_elems_per_mt_opening =
            leaves_per_mt_opening * circuit_field_elems_per_leaf;
        (circuit_field_elems_per_mt_opening / C::SimdCircuitField::PACK_SIZE).ilog2() as usize
    }

    /// NOTE(HS): this is actually number of variables in polynomial,
    /// ignoring the variables for MPI parties and SIMD field element
    fn gen_params(n_input_vars: usize) -> Self::Params {
        n_input_vars
    }

    fn gen_srs_for_testing(
        params: &Self::Params,
        _mpi_config: &MPIConfig,
        rng: impl rand::RngCore,
    ) -> Self::SRS {
        let num_vars_each_core = *params + C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
        OrionSRS::from_random::<C::CircuitField>(
            num_vars_each_core,
            ORION_CODE_PARAMETER_INSTANCE,
            rng,
        )
    }

    fn init_scratch_pad(_params: &Self::Params, _mpi_config: &MPIConfig) -> Self::ScratchPad {
        Self::ScratchPad::default()
    }

    fn commit(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl MultilinearExtension<C::SimdCircuitField>,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Option<Self::Commitment> {
        let num_vars_each_core = *params + C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
        assert_eq!(num_vars_each_core, proving_key.num_vars);

        // NOTE: Hang also assume that, linear GKR will take over the commitment
        // and force sync transcript hash state of subordinate machines to be the same.
        if mpi_config.is_single_process() {
            return orion_commit_simd_field(proving_key, poly, scratch_pad).ok();
        }

        orion_mpi_commit_simd_field(mpi_config, proving_key, poly, scratch_pad).ok()
    }

    fn open(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl MultilinearExtension<C::SimdCircuitField>,
        eval_point: &ExpanderGKRChallenge<C>,
        transcript: &mut T,
        scratch_pad: &Self::ScratchPad,
    ) -> Option<Self::Opening> {
        let num_vars_each_core = *params + C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
        assert_eq!(num_vars_each_core, proving_key.num_vars);

        if mpi_config.is_single_process() {
            let (_, opening) = orion_open_simd_field::<_, C::SimdCircuitField, _, ComPackF, _>(
                proving_key,
                poly,
                &eval_point.local_xs(),
                transcript,
                scratch_pad,
            );
            return opening.into();
        }

        orion_mpi_open_simd_field(
            mpi_config,
            proving_key,
            poly,
            &eval_point.local_xs(),
            &eval_point.x_mpi,
            transcript,
            scratch_pad,
        )
    }

    fn verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        eval_point: &ExpanderGKRChallenge<C>,
        eval: C::ChallengeField,
        transcript: &mut T,
        opening: &Self::Opening,
    ) -> bool {
        orion_verify::<_, C::SimdCircuitField, _, ComPackF, _>(
            verifying_key,
            commitment,
            &eval_point.local_xs(),
            &eval_point.x_mpi,
            eval,
            transcript,
            opening,
        )
    }
}

pub type OrionPCSForGKR<C, ComPack, T> = OrionSIMDFieldPCS<
    <C as GKRFieldConfig>::CircuitField,
    <C as GKRFieldConfig>::SimdCircuitField,
    <C as GKRFieldConfig>::ChallengeField,
    ComPack,
    T,
>;
