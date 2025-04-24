use std::iter;

use arith::{bit_reverse_swap, ExtensionField, FFTField};
use gkr_engine::Transcript;
use itertools::{chain, izip};
use polynomials::{EqPolynomial, MultiLinearPoly, MultilinearExtension, UnivariatePoly};
use serdes::ExpSerde;
use tree::{Node, Tree};

use crate::{
    fri::{
        utils::{copy_elems_to_leaves, fri_alphabets, fri_mt_opening},
        vanilla_sumcheck::{sumcheck_deg_2_mul_step_prove, sumcheck_deg_2_mul_step_verify},
    },
    FRICommitment, FRIOpening, FRIScratchPad,
};

pub(crate) const LOG_CODE_RATE: usize = 2;

const QUERY_COMPLEXITY: usize = 100;

// TODO(HS) query complexity from code rate and field size

#[inline(always)]
pub fn fri_commit<F: FFTField>(
    poly: &impl MultilinearExtension<F>,
    code_rate_log2: usize,
    scratch_pad: &mut FRIScratchPad<F>,
) -> FRICommitment {
    let codeword = {
        let mut temp = poly.hypercube_basis();
        bit_reverse_swap(&mut temp);
        temp.resize(poly.hypercube_size() << LOG_CODE_RATE, F::ZERO);
        F::fft_in_place(&mut temp);
        temp
    };

    let leaves = copy_elems_to_leaves(&codeword);
    let merkle_tree = Tree::new_with_leaves(leaves);

    let commitment = merkle_tree.root();

    scratch_pad.merkle = merkle_tree;
    scratch_pad.codeword = codeword;
    scratch_pad.rate_log2 = code_rate_log2;

    commitment
}

#[inline(always)]
pub fn fri_open<F, ExtF>(
    poly: &impl MultilinearExtension<F>,
    point: &[ExtF],
    fs_transcript: &mut impl Transcript<ExtF>,
    scratch_pad: &FRIScratchPad<F>,
) -> (ExtF, FRIOpening<ExtF>)
where
    F: FFTField + ExpSerde,
    ExtF: ExtensionField + From<F> + ExpSerde + FFTField,
{
    let mut eq_z_poly = MultiLinearPoly::new(EqPolynomial::build_eq_x_r(point));

    let mut ext_poly = MultiLinearPoly::new(
        poly.hypercube_basis_ref()
            .iter()
            .map(|&t| t.into())
            .collect(),
    );

    let num_vars = poly.num_vars();

    let mut iopp_codewords: Vec<Vec<ExtF>> = Vec::with_capacity(num_vars);
    let mut iopp_oracles: Vec<tree::Tree> = Vec::with_capacity(num_vars);

    let mut codeword: Vec<ExtF> = scratch_pad.codeword.iter().map(|&t| t.into()).collect();
    let mut generator = ExtF::two_adic_generator(point.len() + LOG_CODE_RATE);

    let univ_polys: Vec<Vec<ExtF>> = (0..num_vars)
        .map(|i| {
            let (uni_poly, r) =
                sumcheck_deg_2_mul_step_prove(&mut ext_poly, &mut eq_z_poly, fs_transcript);

            let next_codeword_len = codeword.len() / 2;

            let mut diag_inv = ExtF::ONE;
            let one_minus_r = ExtF::ONE - r;
            let generator_inv = generator.inv().unwrap();

            let (odd_alphabets, even_alphabets) = codeword.split_at_mut(next_codeword_len);
            izip!(odd_alphabets, even_alphabets).for_each(|(o_i, e_i)| {
                let o = (*o_i + *e_i) * ExtF::INV_2;
                let e = (*o_i - *e_i) * ExtF::INV_2 * diag_inv;

                *o_i = o * one_minus_r + e * r;
                diag_inv *= generator_inv;
            });
            generator = generator.square();

            codeword.resize(next_codeword_len, ExtF::ZERO);

            if i != num_vars - 1 {
                let leaves = copy_elems_to_leaves(&codeword);
                let merkle_tree = Tree::new_with_leaves(leaves);
                fs_transcript.append_u8_slice(merkle_tree.root().as_bytes());

                iopp_oracles.push(merkle_tree);
                iopp_codewords.push(codeword.clone());
            }

            uni_poly.coeffs
        })
        .collect();

    let challenge_indices = fs_transcript.generate_challenge_index_vector(QUERY_COMPLEXITY);
    let iopp_queries: Vec<Vec<(tree::Path, tree::Path)>> = challenge_indices
        .iter()
        .map(|index| {
            let mut code_len = scratch_pad.codeword.len();
            let mut alphabet_i = index % code_len;

            let mut iopp_round_query = Vec::with_capacity(iopp_oracles.len() + 1);

            let lr_qs = fri_mt_opening(&mut alphabet_i, code_len, &scratch_pad.merkle);

            iopp_round_query.push(lr_qs);
            code_len >>= 1;

            iopp_oracles.iter().for_each(|oracle| {
                let lr_qs = fri_mt_opening(&mut alphabet_i, code_len, oracle);

                iopp_round_query.push(lr_qs);
                code_len >>= 1;
            });

            iopp_round_query
        })
        .collect();

    let eval = {
        let univ_p = UnivariatePoly::new(univ_polys[0].clone());
        univ_p.evaluate(ExtF::ZERO) + univ_p.evaluate(ExtF::ONE)
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

#[inline(always)]
pub fn fri_verify<F, ExtF>(
    com: &FRICommitment,
    point: &[ExtF],
    evaluation: ExtF,
    opening: &FRIOpening<ExtF>,
    fs_transcript: &mut impl Transcript<ExtF>,
) -> bool
where
    F: FFTField + ExpSerde,
    ExtF: ExtensionField + From<F> + ExpSerde + FFTField,
{
    let mut v_claim = evaluation;

    let mut rs: Vec<ExtF> = Vec::new();
    let phony_node = Node::default();
    izip!(
        &opening.sumcheck_responses,
        chain!(&opening.iopp_oracles, iter::once(&phony_node))
    )
    .enumerate()
    .for_each(|(i, (coeffs, oracle))| {
        let univ_p = UnivariatePoly::new(coeffs.clone());
        let r_i = sumcheck_deg_2_mul_step_verify(&mut v_claim, &univ_p, fs_transcript);
        assert!(r_i.is_some());

        v_claim = univ_p.evaluate(r_i.unwrap());
        rs.push(r_i.unwrap());

        if i != point.len() - 1 {
            fs_transcript.append_u8_slice(oracle.as_bytes());
        }
    });

    rs.reverse();

    let f_z = {
        let eq_z_r = EqPolynomial::eq_vec(point, &rs);
        let f_z_eq_z_r = {
            let coeffs = opening.sumcheck_responses.last().unwrap().clone();
            let univ_p = UnivariatePoly::new(coeffs);
            univ_p.evaluate(rs[0])
        };

        f_z_eq_z_r * eq_z_r.inv().unwrap()
    };

    let challenge_indices = fs_transcript.generate_challenge_index_vector(QUERY_COMPLEXITY);
    izip!(&challenge_indices, &opening.iopp_queries).all(|(index, iopp_query)| {
        let mut code_len = 1 << (point.len() + LOG_CODE_RATE);
        let mut alphabet_i = index % code_len;
        let mut generator = ExtF::two_adic_generator(point.len() + LOG_CODE_RATE);

        if !iopp_query[0].0.verify(com) || !iopp_query[0].1.verify(com) {
            return false;
        }

        let (l, r): (ExtF, ExtF) = {
            let (l, r): (F, F) = fri_alphabets(&mut alphabet_i, code_len, &iopp_query[0]);
            (From::from(l), From::from(r))
        };

        let mut expected_next = {
            let diag_inv = generator.exp(alphabet_i as u128).inv().unwrap();

            let r_i = rs.last().unwrap();
            let o = (l + r) * ExtF::INV_2;
            let e = (l - r) * ExtF::INV_2 * diag_inv;

            o * (ExtF::ONE - r_i) + e * r_i
        };

        code_len >>= 1;
        generator = generator.square();
        let mut is_right_query = alphabet_i >= code_len / 2;

        let query_verify = izip!(
            &opening.iopp_oracles,
            rs.iter().rev().skip(1),
            iopp_query.iter().skip(1)
        )
        .all(|(com, r_i, lr_qs)| {
            if !lr_qs.0.verify(com) || !lr_qs.1.verify(com) {
                return false;
            }

            let (l, r): (ExtF, ExtF) = fri_alphabets(&mut alphabet_i, code_len, lr_qs);

            let actual = if is_right_query { r } else { l };
            if actual != expected_next {
                return false;
            }

            expected_next = {
                let diag_inv = generator.exp(alphabet_i as u128).inv().unwrap();

                let o = (l + r) * ExtF::INV_2;
                let e = (l - r) * ExtF::INV_2 * diag_inv;

                o * (ExtF::ONE - r_i) + e * r_i
            };

            code_len >>= 1;
            generator = generator.square();
            is_right_query = alphabet_i >= code_len / 2;

            true
        });

        query_verify && expected_next == f_z
    })
}
