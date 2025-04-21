use arith::{bit_reverse_swap, ExtensionField, FFTField};
use gkr_engine::Transcript;
use itertools::izip;
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

const QUERY_COMPLEXITY: usize = 50;

#[allow(unused)]
pub(crate) fn fri_commit<F: FFTField>(
    coeffs: &[F],
    scratch_pad: &mut FRIScratchPad<F>,
) -> FRICommitment {
    assert!(coeffs.len().is_power_of_two());

    let mut codeword = {
        let mut temp = coeffs.to_vec();
        bit_reverse_swap(&mut temp);
        temp.resize(coeffs.len() << LOG_CODE_RATE, F::ZERO);
        F::fft_in_place(&mut temp);
        temp
    };

    let leaves = copy_elems_to_leaves(&codeword);
    let merkle_tree = Tree::new_with_leaves(leaves);

    let commitment = merkle_tree.root();

    scratch_pad.reed_solomon_commitment = merkle_tree;
    scratch_pad.codeword = codeword;

    commitment
}

#[allow(unused)]
pub(crate) fn fri_open<F, ChallengeF>(
    poly: &impl MultilinearExtension<F>,
    point: &[ChallengeF],
    fs_transcript: &mut impl Transcript<ChallengeF>,
    scratch_pad: &FRIScratchPad<F>,
) where
    F: FFTField + ExpSerde,
    ChallengeF: ExtensionField<BaseField = F> + ExpSerde + FFTField,
{
    let mut shift_z_poly = MultiLinearPoly::new(EqPolynomial::build_eq_x_r(point));

    let mut ext_poly = MultiLinearPoly::new(
        poly.hypercube_basis_ref()
            .iter()
            .cloned()
            .map(From::from)
            .collect(),
    );

    let num_vars = poly.num_vars();

    let mut iopp_codewords: Vec<Vec<ChallengeF>> = Vec::with_capacity(num_vars);
    let mut iopp_oracles: Vec<tree::Tree> = Vec::with_capacity(num_vars);

    let mut codeword: Vec<ChallengeF> = scratch_pad
        .codeword
        .iter()
        .cloned()
        .map(From::from)
        .collect();

    let mut generator = ChallengeF::two_adic_generator(point.len() + LOG_CODE_RATE);
    let two_inv = ChallengeF::ONE.double().inv().unwrap();

    let univ_polys: Vec<UnivariatePoly<ChallengeF>> = (0..num_vars)
        .map(|i| {
            let (uni_poly_i, r_i) = vanilla_sumcheck_degree_2_mul_step_prove(
                &mut ext_poly,
                &mut shift_z_poly,
                fs_transcript,
            );

            let next_codeword_len = codeword.len() / 2;

            let mut diag_inv = ChallengeF::ONE;
            let one_minus_r_i = ChallengeF::ONE - r_i;
            let generator_inv = generator.inv().unwrap();

            let (odd_alphabets, even_alphabets) = codeword.split_at_mut(next_codeword_len);
            izip!(odd_alphabets, even_alphabets).for_each(|(o_i, e_i)| {
                let o = (*o_i + *e_i) * two_inv;
                let e = (*o_i - *e_i) * two_inv * diag_inv;

                *o_i = o * one_minus_r_i + e * r_i;
                diag_inv *= generator_inv;
            });
            generator = generator.square();

            codeword.resize(next_codeword_len, ChallengeF::ZERO);

            let leaves = copy_elems_to_leaves(&codeword);
            let merkle_tree = Tree::new_with_leaves(leaves);
            fs_transcript.append_u8_slice(merkle_tree.root().as_bytes());

            iopp_oracles.push(merkle_tree);
            iopp_codewords.push(codeword.clone());

            uni_poly_i
        })
        .collect();

    dbg!(ext_poly.coeffs[0]);
    dbg!(&iopp_codewords.last());
    assert_eq!(ext_poly.coeffs[0], iopp_codewords.last().unwrap()[0]);

    // TODO(HS) revamped up to here

    return;

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

/*
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
