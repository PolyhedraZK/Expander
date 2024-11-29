use arith::SimdField;
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
        let mut buffer = vec![Self::Commitment::default(); mpi_config.world_size()];
        mpi_config.gather_vec(&local_buffer, &mut buffer);

        // NOTE: Hang also assume that, linear GKR will take over the commitment
        // and force sync transcript hash state of subordinate machines to be the same.
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

        // NOTE: sample MPI linear combination coeffs for proximity rows and eval row
        let mpi_random_coeffs: Vec<_> = (0..local_opening.proximity_rows.len())
            .map(|_| transcript.generate_challenge_field_elements(mpi_config.world_size()))
            .collect();
        let mpi_eq_coeffs = EqPolynomial::build_eq_x_r(&eval_point.x_mpi);

        // NOTE: eval row combine from MPI
        let eval_row = mpi_config.coef_combine_vec(&local_opening.eval_row, &mpi_eq_coeffs);

        // NOTE: proximity rows combine from MPI
        let proximity_rows = local_opening
            .proximity_rows
            .iter()
            .zip(mpi_random_coeffs.iter())
            .map(|(row, weights)| mpi_config.coef_combine_vec(row, weights))
            .collect();

        // NOTE: gather all merkle paths
        let mut query_openings = vec![
            tree::RangePath::default();
            mpi_config.world_size() * local_opening.query_openings.len()
        ];
        mpi_config.gather_vec(&local_opening.query_openings, &mut query_openings);

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
        v: C::ChallengeField,
        transcript: &mut T, // add transcript here to allow interactive arguments
        opening: &Self::Opening,
    ) -> bool {
        let num_vars_each_core = *params - mpi_config.world_size();
        assert_eq!(num_vars_each_core, verifying_key.num_vars);

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

        // NOTE: we now assume that the input opening is from the root machine,
        // as proofs from other machines are typically undefined

        todo!()
    }
}
