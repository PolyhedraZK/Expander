use std::marker::PhantomData;

use arith::{ExtensionField, Field, SimdField};
use ark_std::test_rng;
use gf2::{GF2x128, GF2x8};
use gf2_128::GF2_128;
use gkr_field_config::{GF2ExtConfig, GKRFieldConfig};
use itertools::izip;
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};

use crate::{
    orion::{simd_field_agg_impl::*, *},
    traits::TensorCodeIOPPCS,
    ExpanderGKRChallenge,
};

#[derive(Clone)]
struct DistributedCommitter<F, EvalF, ComPackF, T>
where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    pub scratch_pad: OrionScratchPad<F, ComPackF>,
    pub transcript: T,

    _phantom: PhantomData<EvalF>,
}

fn test_orion_simd_aggregate_verify_helper<C, ComPackF, OpenPackF, T>(
    num_parties: usize,
    num_vars: usize,
) where
    C: GKRFieldConfig,
    ComPackF: SimdField<Scalar = C::CircuitField>,
    OpenPackF: SimdField<Scalar = C::CircuitField>,
    T: Transcript<C::ChallengeField>,
{
    assert!(num_parties.is_power_of_two());

    let mut rng = test_rng();

    let simd_num_vars = C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
    let world_num_vars = num_parties.ilog2() as usize;

    let num_vars_in_unpacked_msg = {
        let (row_field_elems, _) =
            OrionSRS::evals_shape::<C::CircuitField>(num_vars - world_num_vars);
        let row_num = row_field_elems / C::SimdCircuitField::PACK_SIZE;
        let num_vars_in_row = row_num.ilog2() as usize;
        num_vars - world_num_vars - num_vars_in_row
    };

    let global_poly =
        MultiLinearPoly::<C::SimdCircuitField>::random(num_vars - simd_num_vars, &mut rng);

    let global_real_num_vars = global_poly.get_num_vars();
    let local_real_num_vars = global_real_num_vars - world_num_vars;

    let eval_point: Vec<_> = (0..num_vars)
        .map(|_| C::ChallengeField::random_unsafe(&mut rng))
        .collect();

    let gkr_challenge: ExpanderGKRChallenge<C> = ExpanderGKRChallenge {
        x_mpi: eval_point[num_vars - world_num_vars..].to_vec(),
        x: eval_point[simd_num_vars..num_vars - world_num_vars].to_vec(),
        x_simd: eval_point[..simd_num_vars].to_vec(),
    };

    let mut committee = vec![
        DistributedCommitter {
            scratch_pad: OrionScratchPad::<C::CircuitField, ComPackF>::default(),
            transcript: T::new(),
            _phantom: PhantomData,
        };
        num_parties
    ];
    let mut verifier_transcript = T::new();

    let srs = OrionSRS::from_random::<C::CircuitField>(
        num_vars - world_num_vars,
        ORION_CODE_PARAMETER_INSTANCE,
        &mut rng,
    );

    let final_commitment = {
        let roots: Vec<_> = izip!(
            &mut committee,
            global_poly.coeffs.chunks(1 << local_real_num_vars)
        )
        .map(|(committer, eval_slice)| {
            let cloned_poly = MultiLinearPoly::new(eval_slice.to_vec());
            orion_commit_simd_field(&srs, &cloned_poly, &mut committer.scratch_pad).unwrap()
        })
        .collect();

        let final_tree_height = 1 + roots.len().ilog2();
        let (internals, _) = tree::Tree::new_with_leaf_nodes(roots, final_tree_height);
        internals[0]
    };

    let openings: Vec<_> = izip!(
        &mut committee,
        global_poly.coeffs.chunks(1 << local_real_num_vars)
    )
    .map(|(committer, eval_slice)| {
        let cloned_poly = MultiLinearPoly::new(eval_slice.to_vec());
        orion_open_simd_field::<
            C::CircuitField,
            C::SimdCircuitField,
            C::ChallengeField,
            ComPackF,
            OpenPackF,
            T,
        >(
            &srs,
            &cloned_poly,
            &gkr_challenge.local_xs(),
            &mut committer.transcript,
            &committer.scratch_pad,
        )
    })
    .collect();

    let mut aggregator_transcript = committee[0].transcript.clone();
    let aggregated_proof = orion_proof_aggregate::<C, ComPackF, OpenPackF, T>(
        &openings,
        &gkr_challenge.x_mpi,
        &mut aggregator_transcript,
    );

    let mut scratch = vec![C::ChallengeField::ZERO; 1 << num_vars_in_unpacked_msg];
    let final_expected_eval = MultiLinearPoly::evaluate_with_buffer(
        &aggregated_proof.eval_row,
        &gkr_challenge.local_xs()[..num_vars_in_unpacked_msg],
        &mut scratch,
    );

    assert!(orion_verify_simd_field_aggregated::<
        C,
        ComPackF,
        OpenPackF,
        T,
    >(
        num_parties,
        &srs,
        &final_commitment,
        &gkr_challenge,
        final_expected_eval,
        &mut verifier_transcript,
        &aggregated_proof,
    ));
}

#[test]
fn test_orion_simd_aggregate_verify() {
    let parties = 16;

    (25..30).for_each(|num_var| {
        test_orion_simd_aggregate_verify_helper::<
            GF2ExtConfig,
            GF2x128,
            GF2x8,
            BytesHashTranscript<GF2_128, Keccak256hasher>,
        >(parties, num_var)
    })
}
