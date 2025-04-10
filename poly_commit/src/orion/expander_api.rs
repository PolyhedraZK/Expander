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

    /// Minimum number of variables supported by Orion, need world size as input.
    ///
    /// The computation, or simulation, goes as follows:
    /// - On given a world size, we assume the number of variables is as low as possible.
    /// - The minimum query from Merkle tree contains 2 leaves, should be shared across MPI parties.
    /// - NOTE: if the world size is rather large, another factor need to be considered is the
    ///   commitment SIMD field size used in SIMD encoding.  Each party contribute one SIMD field
    ///   element at least to the one query opening, and supposing the world size is too large, we
    ///   have to scale up the opening size.
    /// - Once how many commitment SIMD field elements are contributed into the query opening, we
    ///   just need to ensure the resulting encoded codeword is longer than the world size.
    ///
    /// NOTE: this method will be invoked before setup,
    /// and suggest prover to extend the polynomial size up to the minimal number of variables.
    fn minimum_num_vars(world_size: usize) -> usize {
        const MINIMUM_QUERY_LEAVES: usize = 2;

        let num_bits_com_pack_f = ComPackF::PACK_SIZE * C::CircuitField::FIELD_SIZE;

        let num_compack_fs_per_world_in_query = {
            let minimum_bytes_opening = MINIMUM_QUERY_LEAVES * tree::LEAF_BYTES;
            let minimum_bits_opening = minimum_bytes_opening * 8;

            minimum_bits_opening.div_ceil(world_size * num_bits_com_pack_f)
        };

        let num_simd_fs_per_world_in_query = {
            let relative_pack_size = ComPackF::PACK_SIZE / C::SimdCircuitField::PACK_SIZE;
            num_compack_fs_per_world_in_query * relative_pack_size
        };

        let minimum_msg_size = {
            let min_expander_po2_code_len = ORION_CODE_PARAMETER_INSTANCE
                .length_threshold_g0s
                .next_power_of_two();

            if world_size <= min_expander_po2_code_len {
                world_size
            } else {
                world_size / 2
            }
        };

        (num_simd_fs_per_world_in_query * minimum_msg_size).ilog2() as usize
    }

    /// NOTE(HS): this is actually number of variables in polynomial,
    /// ignoring the variables for MPI parties and SIMD field element
    fn gen_params(n_input_vars: usize) -> Self::Params {
        n_input_vars
    }

    fn gen_srs_for_testing(
        params: &Self::Params,
        mpi_engine: &impl MPIEngine,
        rng: impl rand::RngCore,
    ) -> Self::SRS {
        let num_vars_each_core = *params + C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
        let (srs, _calibrated_num_vars_each_core) = OrionSRS::from_random::<C::CircuitField>(
            mpi_engine.world_size(),
            num_vars_each_core,
            ComPackF::PACK_SIZE,
            ORION_CODE_PARAMETER_INSTANCE,
            rng,
        );
        srs
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

        // NOTE: Hang also assume that, linear GKR will take over the commitment
        // and force sync transcript hash state of subordinate machines to be the same.
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
        verifying_key: &<Self::SRS as gkr_engine::StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &ExpanderSingleVarChallenge<C>,
        v: <C as FieldEngine>::ChallengeField,
        transcript: &mut impl gkr_engine::Transcript<<C as FieldEngine>::ChallengeField>,
        opening: &Self::Opening,
    ) -> bool {
        orion_verify::<_, C::SimdCircuitField, _, ComPackF>(
            verifying_key,
            commitment,
            &x.local_xs(),
            &x.r_mpi,
            v,
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
