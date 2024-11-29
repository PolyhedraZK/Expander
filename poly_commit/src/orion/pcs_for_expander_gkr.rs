use std::io::Cursor;

use arith::{Field, FieldSerde, SimdField};
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use polynomials::{EqPolynomial, MultiLinearPoly};
use transcript::Transcript;

use crate::{orion::*, ExpanderGKRChallenge, PCSForExpanderGKR, StructuredReferenceString};

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

    fn gen_params(n_input_vars: usize) -> Self::Params {
        n_input_vars
    }

    fn gen_srs_for_testing(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        rng: impl rand::RngCore,
    ) -> Self::SRS {
        let num_vars_each_core = *params - mpi_config.world_size();
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
        poly: &MultiLinearPoly<C::SimdCircuitField>,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        let num_vars_each_core = *params - mpi_config.world_size();
        assert_eq!(num_vars_each_core, proving_key.num_vars);

        let commitment = orion_commit_simd_field(proving_key, poly, scratch_pad).unwrap();
        if mpi_config.world_size == 1 {
            return commitment;
        }

        let local_buffer = vec![commitment];
        let mut buffer = match mpi_config.is_root() {
            true => vec![Self::Commitment::default(); mpi_config.world_size()],
            _ => Vec::new(),
        };
        mpi_config.gather_vec(&local_buffer, &mut buffer);

        let mut root = Self::Commitment::default();
        if mpi_config.is_root() {
            let final_tree_height = 1 + buffer.len().ilog2();
            let (internals, _) = tree::Tree::new_with_leaf_nodes(buffer.clone(), final_tree_height);
            root = internals[0];
        }
        mpi_config.root_broadcast_f(&mut root);
        root
    }

    fn open(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &MultiLinearPoly<C::SimdCircuitField>,
        eval_point: &ExpanderGKRChallenge<C>,
        transcript: &mut T, // add transcript here to allow interactive arguments
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Opening {
        let num_vars_each_core = *params - mpi_config.world_size();
        assert_eq!(num_vars_each_core, proving_key.num_vars);

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

        let mpi_random_coeffs: Vec<_> = (0..local_opening.proximity_rows.len())
            .map(|_| transcript.generate_challenge_field_elements(mpi_config.world_size()))
            .collect();
        let mpi_eq_coeffs = EqPolynomial::build_eq_x_r(&eval_point.x_mpi);

        let mut combined_eval_row = local_opening.eval_row.clone();
        mpi_linear_combine(mpi_config, &mut combined_eval_row, &mpi_eq_coeffs);

        let mut combined_proximity_rows = local_opening.proximity_rows.clone();
        combined_proximity_rows
            .iter_mut()
            .zip(mpi_random_coeffs.iter())
            .for_each(|(row, weights)| mpi_linear_combine(mpi_config, row, weights));

        // TODO gather all merkle paths

        todo!()
    }

    fn verify(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        eval_point: &ExpanderGKRChallenge<C>,
        v: C::ChallengeField,
        transcript: &mut T, // add transcript here to allow interactive arguments
        opening: &Self::Opening,
    ) -> bool {
        let num_vars_each_core = *params - mpi_config.world_size();
        assert_eq!(num_vars_each_core, verifying_key.num_vars);

        // TODO only verify the gathered orion opening

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

        // TODO ... decide open and verify in distributed settings

        todo!()
    }
}

fn mpi_linear_combine<F: Field>(mpi_comm: &MPIConfig, local_vec: &mut Vec<F>, weights: &[F]) {
    let combined = mpi_comm.coef_combine_vec(local_vec, weights);

    let mut bytes: Vec<u8> = Vec::new();
    combined.serialize_into(&mut bytes).unwrap();
    mpi_comm.root_broadcast_bytes(&mut bytes);

    let cursor = Cursor::new(bytes);
    let final_res = <Vec<F> as FieldSerde>::deserialize_from(cursor).unwrap();

    local_vec.copy_from_slice(&final_res);
}
