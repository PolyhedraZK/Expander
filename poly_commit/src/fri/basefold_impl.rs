use arith::{ExtensionField, FFTField};
use gkr_engine::Transcript;
use polynomials::{EqPolynomial, MultiLinearPoly, MultilinearExtension, UnivariatePoly};
use serdes::ExpSerde;
use tree::Tree;

use crate::{
    fri::{
        utils::copy_elems_to_leaves, vanilla_sumcheck::vanilla_sumcheck_degree_2_mul_step_prove,
    },
    FRICommitment, FRIScratchPad,
};

const LOG_CODE_RATE: usize = 2;

const MERGE_POLY_DEG: usize = 2;

const QUERY_COMPLEXITY: usize = 50;

#[allow(unused)]
fn fri_commit<F: FFTField>(coeffs: &[F], scratch_pad: &mut FRIScratchPad<F>) -> FRICommitment {
    assert!(coeffs.len().is_power_of_two());

    let mut codeword = {
        let mle = MultiLinearPoly::new(coeffs.to_vec());
        mle.interpolate_over_hypercube()
    };
    codeword.resize(coeffs.len() << LOG_CODE_RATE, F::ZERO);

    F::fft_in_place(&mut codeword);

    let leaves = copy_elems_to_leaves(&codeword);
    let merkle_tree = Tree::new_with_leaves(leaves);

    let commitment = merkle_tree.root();

    scratch_pad.reed_solomon_commitment = merkle_tree;
    scratch_pad.codeword = codeword;

    commitment
}

/*
#[allow(unused)]
fn fri_open<F, ChallengeF>(
    poly: &impl MultilinearExtension<F>,
    point: &[ChallengeF],
    fs_transcript: &mut impl Transcript<ChallengeF>,
    scratch_pad: &FRIScratchPad<F>,
) where
    F: FFTField + ExpSerde,
    ChallengeF: ExtensionField<BaseField = F> + ExpSerde + FFTField,
{
    let shift_z_poly = MultiLinearPoly::new(EqPolynomial::build_eq_x_r(point));

    let ext_poly = MultiLinearPoly::new(
        poly.hypercube_basis_ref()
            .iter()
            .cloned()
            .map(From::from)
            .collect(),
    );
    let merge_function = |x: &[ChallengeF]| x.iter().product::<ChallengeF>();

    let num_vars = poly.num_vars();

    let mut sumcheck_polys: Vec<UnivariatePoly<ChallengeF>> = Vec::with_capacity(num_vars);
    let mut iopp_codewords: Vec<Vec<ChallengeF>> = Vec::with_capacity(num_vars);
    let mut iopp_oracles: Vec<tree::Tree> = Vec::with_capacity(num_vars);

    // todo: merge this loop into the sumcheck protocol.
    let rs = (0..num_vars)
        .flat_map(|i| {
            // NOTE: sumcheck a single step, r_i start from x_0 towards x_n
            // TODO: this seems to sumcheck against a product of two polynomials.
            // Try to use our own sumcheck instead
            let (sc_univariate_poly_i, r_i, next_claim) = vanilla_sumcheck_degree_2_mul_step_prove(
                &mut ext_poly,
                &mut shift_z_poly,
                fs_transcript,
            );
            sumcheck_polys.push(sc_univariate_poly_i.uni_polys[0].clone());
            drop(sc_univariate_poly_i);

            let mut coeffs = sumcheck_poly_vec[0].interpolate_over_hypercube();
            coeffs.resize(coeffs.len() << LOG_CODE_RATE, ChallengeF::ZERO);
            ChallengeF::fft_in_place(&mut coeffs);

            {
                let leaves = copy_elems_to_leaves(&coeffs);
                let merkle_tree = Tree::new_with_leaves(leaves);
                iopp_oracles.push(merkle_tree)
            };

            iopp_codewords.push(coeffs.clone());

            // println!("{}-th round: randomness: {:?}", i, rs);
            println!("{}-th round: final evals: {:?}", i, next_claim);
            r_i
        })
        .collect::<Vec<ChallengeF>>();

    println!("prover randomness: {:?}", rs);

    let eq_zr_2 = EqPolynomial::eq_vec(&rs, point);
    println!("prover eq(z, r): {:?}", eq_zr_2);

    let mut scratch = vec![ChallengeF::ZERO; poly.hypercube_size()];

    let poly_r = ext_poly.evaluate_with_buffer(&rs, &mut scratch);
    println!("prover f(r): {:?}", poly_r);

    let poly_z = ext_poly.evaluate_with_buffer(point, &mut scratch);
    println!("prover f(z): {:?}", poly_z);

    let iopp_last_oracle_message = iopp_oracles[iopp_oracles.len() - 1].leaves.clone();
    let mut iopp_challenges = fs_transcript.generate_challenge_index_vector(QUERY_COMPLEXITY);
    let mut first_round_queries = vec![];

    let rest_iopp_queries = iopp_challenges
        .iter_mut()
        .map(|mut point| {
            let mut iopp_round_query = Vec::with_capacity(iopp_oracles.len() + 1);

            // Merkle queries over F
            let oracle_rhs_start = scratch_pad.reed_solomon_commitment.size() >> 1;
            let sibling_point = *point ^ oracle_rhs_start;
            let left = std::cmp::min(*point, sibling_point);
            let right = oracle_rhs_start + left;
            *point = left;

            let first_round_query = (
                scratch_pad
                    .reed_solomon_commitment
                    .gen_proof(left, scratch_pad.reed_solomon_commitment.height()),
                scratch_pad
                    .reed_solomon_commitment
                    .gen_proof(right, scratch_pad.reed_solomon_commitment.height()),
            );

            first_round_queries.push(first_round_query);

            // Merkle queries over ExtF
            iopp_oracles.iter().for_each(|oracle| {
                // NOTE: since the oracle length is always a power of 2,
                // so the right hand side starts from directly div by 2.
                let oracle_rhs_start = oracle.size() >> 1;

                // NOTE: dirty trick, oracle rhs starting index is a pow of 2.
                // now that we want to find a sibling point w.r.t the index,
                // by plus (or minus) against point, so xor should work.
                let sibling_point = *point ^ oracle_rhs_start;

                let left = std::cmp::min(*point, sibling_point);
                let right = oracle_rhs_start + left;

                // NOTE: update point for next round of IOPP querying
                *point = left;

                iopp_round_query.push((
                    oracle.gen_proof(left, oracle.height()),
                    oracle.gen_proof(right, oracle.height()),
                ))
            });

            // todo: include first round query in the iopp round query
            iopp_round_query
        })
        .collect::<Vec<_>>();

    /*
    BasefoldProof {
        sumcheck_transcript: SumcheckInstanceProof::new(sumcheck_polys),
        iopp_oracles: iopp_oracles.iter().map(|t| t.root()).collect(),
        iopp_last_oracle_message,
        first_iopp_query: first_round_queries,
        randomness: rs,
        iopp_queries: rest_iopp_queries,
    }
    */
}

#[allow(unused)]
fn fri_verify<F, ChallengeF>(
    commitment: &FRICommitment,
    point: &[ChallengeF],
    evaluation: ChallengeF,
    fs_transcript: &mut impl Transcript<ChallengeF>,
) where
    F: FFTField,
    ChallengeF: ExtensionField<BaseField = F>,
{
}
*/
