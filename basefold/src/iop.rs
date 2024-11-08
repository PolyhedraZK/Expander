use arith::{ExtensionField, FFTField, Field, FieldSerde};
use babybear::BabyBearx16;
use itertools::Itertools;
use transcript::Transcript;
// use p3_baby_bear::PackedBabyBearAVX512 as BabyBearx16;
use tree::{Node, Path};

use crate::BasefoldParam;

#[derive(Clone, Debug, PartialEq)]
pub struct BasefoldIOPPQuery<F: Field + FieldSerde> {
    // NOTE: the folding r's are in sumcheck verification, deriving from Fiat-Shamir.
    pub(crate) iopp_round_query: Vec<BasefoldIOPPQuerySingleRound<F>>,
}

impl<F: FFTField + FieldSerde> BasefoldIOPPQuery<F> {
    fn verify_iopp_query<T, ExtF>(
        iopp_round_query: &[BasefoldIOPPQuerySingleRound<F>],
        setup: &BasefoldParam<T, ExtF, F>,
        challenge_point: usize,
        oracles: &[Node],
        folding_rs: &[ExtF::BaseField],
        is_leading_over_f: bool,
    ) -> bool
    where
        T: Transcript<F>,
        ExtF: ExtensionField<BaseField = F>,
    {
        let num_vars = folding_rs.len();

        let ((iopp_round_query_f, oracles_f), (iopp_round_query_extf, oracles_extf)) =
            match is_leading_over_f {
                true => (
                    (&iopp_round_query[..1], &oracles[..1]),
                    (&iopp_round_query[1..], &oracles[1..]),
                ),
                _ => (
                    (&iopp_round_query[..0], &oracles[..0]),
                    (&iopp_round_query[0..], &oracles[0..]),
                ),
            };

        // check merkle trees against base field or extension field
        let mt_verify_f =
            iopp_round_query_f
                .iter()
                .zip(oracles_f)
                .all(|(round_i_query, root_i)| {

                    println!("round_i left: {}", round_i_query.left);
                    println!("round_i right: {}", round_i_query.right);
                    println!("root_i: {}", root_i);


                    let left = round_i_query.left.verify(root_i);
                    let right = round_i_query.right.verify(root_i);

                    left && right
                });
                
        if !mt_verify_f {
            return false;
        }

        // let mt_verify_extf =
        //     iopp_round_query_extf
        //         .iter()
        //         .zip(oracles_extf)
        //         .all(|(round_i_query, root_i)| {
        //             let left = round_i_query.left.verify(root_i);
        //             let right = round_i_query.right.verify(root_i);

        //             left && right
        //         });

        // if !mt_verify_f || !mt_verify_extf {
        //     return false;
        // }

        // Check IOPP query results
        let iopp_query_f_res_iter = iopp_round_query_f.iter().map(|q| {
            let l_f = q.left.leaf().data;
            let r_f = q.right.leaf().data;

            (l_f, r_f)

            // let l_ef: ExtF = <F>::into(l_f);
            // let r_ef: ExtF = <F>::into(r_f);

            // (l_ef, r_ef)
        });

        let iopp_query_ef_res_iter = iopp_round_query_extf
            .iter()
            .map(|q| (q.left.leaf().data, q.right.leaf().data));

        let iopp_query_res_iter = iopp_query_f_res_iter.chain(iopp_query_ef_res_iter);

        let mut point = challenge_point;
        iopp_query_res_iter
            .tuple_windows()
            .enumerate()
            .all(|(round_i, ((c1, c2), (nc1, nc2)))| {
                let oracle_rhs_start = 1 << (setup.codeword_bits(num_vars) - round_i - 1);
                let sibling_point = point ^ oracle_rhs_start;
                let left_index = std::cmp::min(point, sibling_point);

                let g1 = setup.t_term(num_vars, round_i, left_index);
                let g2 = -g1;

                // interpolate y = b + kx form
                let k = (c2 - c1) * (g2 - g1).inv().unwrap();
                let b = c1 - k * g1;
                let expected_codeword = b + k * folding_rs[round_i];

                point = left_index;

                let actual_codeword = match left_index & (oracle_rhs_start >> 1) {
                    0 => nc1,
                    _ => nc2,
                };

                actual_codeword == expected_codeword
            })
    }

    pub fn verify<T, ExtF>(
        &self,
        setup: &BasefoldParam<T, ExtF, F>,
        challenge_point: usize,
        oracles: &[Node],
        folding_rs: &[ExtF::BaseField],
    ) -> bool
    where
        T: Transcript<F>,
        ExtF: ExtensionField<BaseField = F>,
    {
        Self::verify_iopp_query(
            &self.iopp_round_query,
            setup,
            challenge_point,
            oracles,
            folding_rs,
            true,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BasefoldIOPPQuerySingleRound<F: Field + FieldSerde> {
    pub(crate) left: Path<F>,
    pub(crate) right: Path<F>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BasefoldVirtualIOPPQuery<F: Field + FieldSerde> {
    virtual_queries: Vec<BasefoldIOPPQuerySingleRound<F>>,
    iopp_query: BasefoldIOPPQuery<F>,
}

impl<F: Field + FieldSerde> BasefoldIOPPQuerySingleRound<F> {
    pub fn check_expected_codeword(
        &self,
        entry_index: usize,
        oracle_len: usize,
        entry: &F,
    ) -> bool {
        let side = &match entry_index & (oracle_len >> 1) {
            0 => self.left.leaf(),
            _ => self.right.leaf(),
        };

        side.data == *entry
    }
}

impl<F: Field + FieldSerde> BasefoldVirtualIOPPQuery<F> {
    #[inline]
    fn deteriorate_to_basefold_iopp_query(&self) -> BasefoldIOPPQuery<F> {
        // NOTE: the deterioration happens only when there is only one virtual query,
        // namely, using batch for one single polynomial.
        assert_eq!(self.virtual_queries.len(), 1);

        let mut iopp_round_query = self.virtual_queries.clone();
        iopp_round_query.extend_from_slice(&self.iopp_query.iopp_round_query);
        BasefoldIOPPQuery { iopp_round_query }
    }
}
