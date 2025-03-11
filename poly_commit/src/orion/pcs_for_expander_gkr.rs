use std::io::Cursor;

use arith::{FieldSerde, SimdField};
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use polynomials::{EqPolynomial, MultilinearExtension};
use transcript::Transcript;

use crate::{
    orion::{simd_field_agg_impl::*, *},
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

    /// NOTE(HS): this is actually number of variables in polynomial,
    /// ignoring the variables for MPI parties and SIMD field element
    fn gen_params(n_input_vars: usize) -> Self::Params {
        n_input_vars
    }

    fn gen_srs_for_testing(
        params: &Self::Params,
        #[allow(unused)] mpi_config: &MPIConfig,
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
    ) -> Self::Commitment {
        let num_vars_each_core = *params + C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
        assert_eq!(num_vars_each_core, proving_key.num_vars);

        let commitment = orion_commit_simd_field(proving_key, poly, scratch_pad).unwrap();

        // NOTE: Hang also assume that, linear GKR will take over the commitment
        // and force sync transcript hash state of subordinate machines to be the same.
        if mpi_config.world_size() == 1 {
            return commitment;
        }

        let local_buffer = vec![commitment];
        let mut buffer = vec![Self::Commitment::default(); mpi_config.world_size()];
        mpi_config.gather_vec(&local_buffer, &mut buffer);

        if !mpi_config.is_root() {
            return commitment;
        }

        let final_tree_height = 1 + buffer.len().ilog2();
        let (internals, _) = tree::Tree::new_with_leaf_nodes(buffer.clone(), final_tree_height);
        internals[0]
    }

    fn open(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl MultilinearExtension<C::SimdCircuitField>,
        eval_point: &ExpanderGKRChallenge<C>,
        transcript: &mut T, // add transcript here to allow interactive arguments
        scratch_pad: &Self::ScratchPad,
    ) -> Self::Opening {
        let num_vars_each_core = *params + C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
        assert_eq!(num_vars_each_core, proving_key.num_vars);

        let local_xs = eval_point.local_xs();
        let local_opening = orion_open_simd_field::<
            C::CircuitField,
            C::SimdCircuitField,
            C::ChallengeField,
            ComPackF,
            T,
        >(proving_key, poly, &local_xs, transcript, scratch_pad);
        if mpi_config.world_size() == 1 {
            return local_opening;
        }

        // NOTE: eval row combine from MPI
        let mpi_eq_coeffs = EqPolynomial::build_eq_x_r(&eval_point.x_mpi);
        let eval_row = mpi_config.coef_combine_vec(&local_opening.eval_row, &mpi_eq_coeffs);

        // NOTE: sample MPI linear combination coeffs for proximity rows,
        // and proximity rows combine with MPI
        let proximity_rows = local_opening
            .proximity_rows
            .iter()
            .map(|row| {
                let weights = transcript.generate_challenge_field_elements(mpi_config.world_size());
                mpi_config.coef_combine_vec(row, &weights)
            })
            .collect();

        // NOTE: local query openings serialized to bytes
        let mut local_mt_paths_serialized = Vec::new();
        local_opening
            .query_openings
            .serialize_into(&mut local_mt_paths_serialized)
            .unwrap();

        // NOTE: Hang does not think this is a good move, but this is mostly
        // working with MPI behavior, so we align local MT openings serialization
        // against power-of-2 bytes length.
        local_mt_paths_serialized.resize(local_mt_paths_serialized.len().next_power_of_two(), 0u8);

        // NOTE: gather all merkle paths
        let mut mt_paths_serialized =
            vec![0u8; mpi_config.world_size() * local_mt_paths_serialized.len()];
        mpi_config.gather_vec(&local_mt_paths_serialized, &mut mt_paths_serialized);

        let query_openings: Vec<tree::RangePath> = mt_paths_serialized
            .chunks(local_mt_paths_serialized.len())
            .flat_map(|bs| {
                let mut read_cursor = Cursor::new(bs);
                Vec::deserialize_from(&mut read_cursor).unwrap()
            })
            .collect();

        if !mpi_config.is_root() {
            return local_opening;
        }

        // NOTE: we only care about the root machine's opening as final proof, Hang assume.
        OrionProof {
            eval_row,
            proximity_rows,
            query_openings,
        }
    }

    fn verify(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        eval_point: &ExpanderGKRChallenge<C>,
        eval: C::ChallengeField,
        transcript: &mut T, // add transcript here to allow interactive arguments
        opening: &Self::Opening,
    ) -> bool {
        let global_poly_num_vars = *params
            + mpi_config.world_size().ilog2() as usize
            + C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
        assert_eq!(global_poly_num_vars, eval_point.num_vars());

        if mpi_config.world_size() == 1 || !mpi_config.is_root() {
            return orion_verify_simd_field::<
                C::CircuitField,
                C::SimdCircuitField,
                C::ChallengeField,
                ComPackF,
                T,
            >(
                verifying_key,
                commitment,
                &eval_point.local_xs(),
                eval,
                transcript,
                opening,
            );
        }

        // NOTE: we now assume that the input opening is from the root machine,
        // as proofs from other machines are typically undefined
        orion_verify_simd_field_aggregated::<C, ComPackF, T>(
            mpi_config.world_size(),
            verifying_key,
            commitment,
            eval_point,
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
