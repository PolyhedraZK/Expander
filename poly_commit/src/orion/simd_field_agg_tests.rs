use std::marker::PhantomData;

use arith::{ExtensionField, Field, SimdField};
use ark_std::test_rng;
use gf2::GF2x128;
use gf2_128::GF2_128;
use gkr_field_config::{GF2ExtConfig, GKRFieldConfig, M31ExtConfig};
use itertools::izip;
use mersenne31::{M31Ext3, M31x16};
use polynomials::{EqPolynomial, MultiLinearPoly, MultiLinearPolyExpander};
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};

use crate::{
    orion::{simd_field_agg_impl::*, utils::*, *},
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

fn orion_proof_aggregate<C, T>(
    openings: &[OrionProof<C::ChallengeField>],
    x_mpi: &[C::ChallengeField],
    transcript: &mut T,
) -> OrionProof<C::ChallengeField>
where
    C: GKRFieldConfig,
    T: Transcript<C::ChallengeField>,
{
    let paths = openings
        .iter()
        .flat_map(|o| o.query_openings.clone())
        .collect();
    let num_parties = 1 << x_mpi.len();

    let proximity_reps = openings[0].proximity_rows.len();
    let mut scratch = vec![C::ChallengeField::ZERO; num_parties * openings[0].eval_row.len()];

    let aggregated_proximity_rows = (0..proximity_reps)
        .map(|i| {
            let weights = transcript.generate_challenge_field_elements(num_parties);
            let mut rows: Vec<_> = openings
                .iter()
                .flat_map(|o| o.proximity_rows[i].clone())
                .collect();
            transpose_in_place(&mut rows, &mut scratch, num_parties);
            rows.chunks(num_parties)
                .map(|c| izip!(c, &weights).map(|(&l, &r)| l * r).sum())
                .collect()
        })
        .collect();

    let aggregated_eval_row: Vec<_> = {
        let eq_worlds_coeffs = EqPolynomial::build_eq_x_r(x_mpi);
        let mut rows: Vec<_> = openings.iter().flat_map(|o| o.eval_row.clone()).collect();
        transpose_in_place(&mut rows, &mut scratch, num_parties);
        rows.chunks(num_parties)
            .map(|c| izip!(c, &eq_worlds_coeffs).map(|(&l, &r)| l * r).sum())
            .collect()
    };

    OrionProof {
        eval_row: aggregated_eval_row,
        proximity_rows: aggregated_proximity_rows,
        query_openings: paths,
    }
}

fn test_orion_simd_aggregate_verify_helper<C, ComPackF, T>(num_parties: usize, num_vars: usize)
where
    C: GKRFieldConfig,
    ComPackF: SimdField<Scalar = C::CircuitField>,
    T: Transcript<C::ChallengeField>,
{
    assert!(num_parties.is_power_of_two());

    let mut rng = test_rng();

    let simd_num_vars = C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
    let world_num_vars = num_parties.ilog2() as usize;

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

    let openings: Vec<_> =
        izip!(
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
    let aggregated_proof =
        orion_proof_aggregate::<C, T>(&openings, &gkr_challenge.x_mpi, &mut aggregator_transcript);

    let final_expected_eval =
        MultiLinearPolyExpander::<C>::single_core_eval_circuit_vals_at_expander_challenge(
            &global_poly.coeffs,
            &gkr_challenge.x,
            &gkr_challenge.x_simd,
            &gkr_challenge.x_mpi,
        );

    assert!(orion_verify_simd_field_aggregated::<C, ComPackF, T>(
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

    (16..18).for_each(|num_var| {
        test_orion_simd_aggregate_verify_helper::<
            GF2ExtConfig,
            GF2x128,
            BytesHashTranscript<GF2_128, Keccak256hasher>,
        >(parties, num_var)
    });

    (12..15).for_each(|num_var| {
        test_orion_simd_aggregate_verify_helper::<
            M31ExtConfig,
            M31x16,
            BytesHashTranscript<M31Ext3, Keccak256hasher>,
        >(parties, num_var)
    })
}
