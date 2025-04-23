use std::iter;

use arith::{bit_reverse_swap, ExtensionField, FFTField, Field};
use gkr_engine::Transcript;
use itertools::izip;
use polynomials::{EqPolynomial, MultiLinearPoly, MultilinearExtension, UnivariatePoly};
use serdes::ExpSerde;
use tree::Tree;

use crate::{
    fri::{
        utils::{copy_elems_to_leaves, fri_fold_step, fri_mt_opening},
        vanilla_sumcheck::{
            vanilla_sumcheck_degree_2_mul_step_prove, vanilla_sumcheck_degree_2_mul_step_verify,
        },
    },
    FRICommitment, FRIScratchPad,
};

const LOG_CODE_RATE: usize = 2;

const QUERY_COMPLEXITY: usize = 100;

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

    scratch_pad.merkle = merkle_tree;
    scratch_pad.codeword = codeword;

    commitment
}

#[allow(unused)]
pub struct FRIOpening<F: Field> {
    pub iopp_oracles: Vec<tree::Node>,
    pub iopp_queries: Vec<Vec<(tree::Path, tree::Path)>>,
    pub sumcheck_responses: Vec<UnivariatePoly<F>>,
}

#[allow(unused)]
pub(crate) fn fri_open<F, ChallengeF>(
    poly: &impl MultilinearExtension<F>,
    point: &[ChallengeF],
    fs_transcript: &mut impl Transcript<ChallengeF>,
    scratch_pad: &FRIScratchPad<F>,
) -> (ChallengeF, FRIOpening<ChallengeF>)
where
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

    let univ_polys: Vec<UnivariatePoly<ChallengeF>> = (0..num_vars)
        .map(|_| {
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
                let o = (*o_i + *e_i) * ChallengeF::INV_2;
                let e = (*o_i - *e_i) * ChallengeF::INV_2 * diag_inv;

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

    let iopp_challenges = fs_transcript.generate_challenge_index_vector(QUERY_COMPLEXITY);
    let iopp_queries: Vec<Vec<(tree::Path, tree::Path)>> = iopp_challenges
        .iter()
        .map(|point| {
            let mut codeword_len = scratch_pad.codeword.len();
            let mut point_to_alphabet = point % codeword_len;

            let mut iopp_round_query = Vec::with_capacity(iopp_oracles.len() + 1);

            let round_opening =
                fri_mt_opening(&mut point_to_alphabet, codeword_len, &scratch_pad.merkle);

            iopp_round_query.push(round_opening);
            codeword_len >>= 1;

            iopp_oracles.iter().for_each(|oracle| {
                let round_opening = fri_mt_opening(&mut point_to_alphabet, codeword_len, oracle);

                iopp_round_query.push(round_opening);
                codeword_len >>= 1;
            });

            iopp_round_query
        })
        .collect();

    let eval = {
        let univ_p = &univ_polys[0];
        univ_p.evaluate(ChallengeF::ZERO) + univ_p.evaluate(ChallengeF::ONE)
    };

    (
        eval,
        FRIOpening {
            iopp_oracles: iopp_oracles.iter().map(|t| t.root()).collect(),
            iopp_queries,
            sumcheck_responses: univ_polys,
        },
    )
}

#[allow(unused)]
pub(crate) fn fri_verify<F, ChallengeF>(
    commitment: &FRICommitment,
    point: &[ChallengeF],
    evaluation: ChallengeF,
    opening: &FRIOpening<ChallengeF>,
    fs_transcript: &mut impl Transcript<ChallengeF>,
) -> bool
where
    F: FFTField + ExpSerde,
    ChallengeF: ExtensionField<BaseField = F> + ExpSerde + FFTField,
{
    let mut v_claim = evaluation;

    let mut rs: Vec<ChallengeF> = Vec::new();
    izip!(&opening.sumcheck_responses, &opening.iopp_oracles).for_each(|(univ_p, oracle)| {
        let r_i = vanilla_sumcheck_degree_2_mul_step_verify(&mut v_claim, univ_p, fs_transcript);
        assert!(r_i.is_some());

        v_claim = univ_p.evaluate(r_i.unwrap());
        rs.push(r_i.unwrap());
        fs_transcript.append_u8_slice(oracle.as_bytes());
    });

    rs.reverse();

    let f_z = {
        let eq_z_r = EqPolynomial::eq_vec(point, &rs);
        let f_z_eq_z_r = {
            let univ_p = opening.sumcheck_responses.last().unwrap();
            univ_p.evaluate(rs[0])
        };

        f_z_eq_z_r * eq_z_r.inv().unwrap()
    };

    dbg!(f_z);

    let last_oracle = {
        let last_codeword = vec![f_z; 1 << LOG_CODE_RATE];
        let leaves = copy_elems_to_leaves(&last_codeword);
        let merkle_tree = Tree::new_with_leaves(leaves);
        merkle_tree.root()
    };

    if last_oracle != *opening.iopp_oracles.last().unwrap() {
        return false;
    }

    let mut fri_verify = true;

    let iopp_challenges = fs_transcript.generate_challenge_index_vector(QUERY_COMPLEXITY);
    izip!(&iopp_challenges, &opening.iopp_queries).for_each(|(challenge, iopp_query)| {
        let mut codeword_len = 1 << (point.len() + LOG_CODE_RATE);
        let mut point_to_alphabet = challenge % codeword_len;
        let mut generator = ChallengeF::two_adic_generator(point.len() + LOG_CODE_RATE);

        fri_verify = fri_verify && iopp_query[0].0.verify(commitment);
        fri_verify = fri_verify && iopp_query[0].1.verify(commitment);

        let (left, right): (ChallengeF, ChallengeF) = {
            let (l, r): (F, F) =
                fri_fold_step(&mut point_to_alphabet, codeword_len, &iopp_query[0]);

            (From::from(l), From::from(r))
        };

        dbg!(left, right);

        let mut expected_next = {
            let diag_inv = generator.exp(point_to_alphabet as u128).inv().unwrap();

            let r_i = rs.last().unwrap();
            let o = (left + right) * ChallengeF::INV_2;
            let e = (left - right) * ChallengeF::INV_2 * diag_inv;

            o * (ChallengeF::ONE - r_i) + e * r_i
        };
        dbg!(expected_next);

        codeword_len >>= 1;
        generator = generator.square();

        let mut is_right_query = point_to_alphabet >= codeword_len / 2;
        dbg!(is_right_query);

        izip!(
            &opening.iopp_oracles,
            rs.iter().rev().skip(1).chain(iter::once(&ChallengeF::ZERO)),
            iopp_query.iter().skip(1)
        )
        .for_each(|(com, r_i, query_pair)| {
            fri_verify = fri_verify && query_pair.0.verify(com) && query_pair.1.verify(com);

            let (left, right): (ChallengeF, ChallengeF) =
                fri_fold_step(&mut point_to_alphabet, codeword_len, query_pair);

            dbg!(left, right);

            let actual = if is_right_query { right } else { left };
            fri_verify = fri_verify && actual == expected_next;

            expected_next = {
                let diag_inv = generator.exp(point_to_alphabet as u128).inv().unwrap();

                let o = (left + right) * ChallengeF::INV_2;
                let e = (left - right) * ChallengeF::INV_2 * diag_inv;

                o * (ChallengeF::ONE - r_i) + e * r_i
            };
            dbg!(expected_next);

            codeword_len >>= 1;
            generator = generator.square();

            is_right_query = point_to_alphabet >= codeword_len / 2;
            dbg!(is_right_query);
        });
    });

    fri_verify
}
