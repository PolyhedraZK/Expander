use std::ops::Mul;

use arith::{ExtensionField, FFTField, FieldSerde};
use ark_std::{end_timer, start_timer};
use babybear::BabyBearx16;
use polynomials::{EqPolynomial, MultiLinearPoly};
// use p3_baby_bear::PackedBabyBearAVX512 as BabyBearx16;
use rand::RngCore;
use sumcheck::SumcheckInstanceProof;
// use sumcheck::SumcheckInstanceProof;
use transcript::{FiatShamirBytesHash, Transcript};
use tree::Tree;

use crate::{
    iop::BasefoldIOPPQuery, BasefoldCommitment, BasefoldIOPPQuerySingleRound, BasefoldParam,
    BasefoldProof, PolynomialCommitmentScheme, LOG_RATE, MERGE_POLY_DEG,
};

pub struct BaseFoldPCS<T, ExtF, F> {
    pub transcript: std::marker::PhantomData<T>,
    // pub hasher: std::marker::PhantomData<H>,
    pub field: std::marker::PhantomData<F>,
    pub ext_field: std::marker::PhantomData<ExtF>,
}

impl<T, ExtF, F> PolynomialCommitmentScheme for BaseFoldPCS<T, ExtF, F>
where
    T: Transcript<F>,
    F: FFTField + FieldSerde,
    ExtF: ExtensionField<BaseField = F>,
{
    type ProverParam = BasefoldParam<T, ExtF, F>;
    type VerifierParam = BasefoldParam<T, ExtF, F>;
    type SRS = BasefoldParam<T, ExtF, F>;
    type Polynomial = MultiLinearPoly<F>;
    type Point = Vec<F>;
    type Evaluation = F;
    type Commitment = BasefoldCommitment<F>;
    type Proof = BasefoldProof<ExtF>;
    type BatchProof = ();
    type Transcript = T;

    fn gen_srs_for_testing(
        _rng: impl RngCore,
        _supported_n: usize,
        _supported_m: usize,
    ) -> Self::SRS {
        BasefoldParam::<T, ExtF, F>::new(LOG_RATE)
    }

    fn commit(
        prover_param: impl std::borrow::Borrow<Self::ProverParam>,
        polynomial: &Self::Polynomial,
    ) -> Self::Commitment {
        let timer = start_timer!(|| format!(
            "basefold commit poly with {} vars",
            polynomial.get_num_vars()
        ));
        let commit = BasefoldCommitment {
            tree: prover_param.borrow().basefold_oracle_from_poly(polynomial),
        };
        end_timer!(timer);

        commit
    }

    fn open(
        prover_param: impl std::borrow::Borrow<Self::ProverParam>,
        commitment: &Self::Commitment,
        polynomial: &Self::Polynomial,
        opening_point: &Self::Point,
        transcript: &mut T,
    ) -> Self::Proof {
        let timer = start_timer!(|| format!(
            "basefold prove poly with {} vars",
            polynomial.get_num_vars()
        ));

        println!("commitment {}", commitment.tree);

        let shift_z = EqPolynomial::build_eq_x_r(
            &opening_point, // .iter()
                            // .map(|&x| F::from(x))
                            // .collect::<Vec<_>>(),
        );

        println!("shift_z: {:?}", shift_z);

        let shift_z_poly = MultiLinearPoly { coeffs: shift_z };
        // let poly_ext_coeff = polynomial.clone();
        //     // .coeffs
        //     // .iter()
        //     // .map(|&x| ExtF::from(x))
        //     // .collect::<Vec<_>>();
        // let poly_ext = MultiLinearPoly {
        //     coeffs: poly_ext_coeff,
        // };

        let mut sumcheck_poly_vec = vec![polynomial.clone(), shift_z_poly.clone()];
        let merge_function = |x: &[F]| x.iter().product::<F>();

        let num_vars = polynomial.get_num_vars();

        let mut sumcheck_polys: Vec<_> = Vec::with_capacity(num_vars);
        let mut iopp_codewords: Vec<Vec<F>> = Vec::with_capacity(num_vars);

        // todo: merge this loop into the sumcheck protocol.
        let rs = (0..num_vars)
            .flat_map(|i| {
                // NOTE: sumcheck a single step, r_i start from x_0 towards x_n
                // TODO: this seems to sumcheck against a product of two polynomials.
                // Try to use our own sumcheck instead
                let (sc_univariate_poly_i, rs, final_evals) =
                    SumcheckInstanceProof::prove_arbitrary(
                        &F::zero(),
                        1,
                        &mut sumcheck_poly_vec,
                        merge_function,
                        MERGE_POLY_DEG,
                        transcript,
                    );
                sumcheck_polys.push(sc_univariate_poly_i.uni_polys[0].clone());
                drop(sc_univariate_poly_i);

                let coeffs = sumcheck_poly_vec[0].interpolate_over_hypercube();
                iopp_codewords.push(prover_param.borrow().reed_solomon_from_coeffs(coeffs));

                // println!("{}-th round: randomness: {:?}", i, rs);
                println!("{}-th round: final evals: {:?}", i, final_evals);
                rs
            })
            .collect::<Vec<F>>();

        println!("prover randomness: {:?}", rs);

        let eq_zr_2 = shift_z_poly.evaluate(&rs);
        println!("prover eq(z, r): {:?}", eq_zr_2);

        let poly_r = polynomial.evaluate(&rs);
        println!("prover f(r): {:?}", poly_r);

        let poly_z = polynomial.evaluate(&opening_point);
        println!("prover f(z): {:?}", poly_z);

        // println!("iopp code wrd: {:?}", iopp_codewords);

        let iopp_oracles = Tree::batch_tree_for_recursive_oracles(iopp_codewords);

        // println!("iopp code word: {:?}", iopp_oracles);

        let iopp_last_oracle_message = iopp_oracles[iopp_oracles.len() - 1].leaves.clone();
        let iopp_challenges = prover_param.borrow().iopp_challenges(num_vars, transcript);
        let mut first_round_queries = vec![];

        let rest_iopp_queries = (0..prover_param.borrow().verifier_queries)
            .zip(iopp_challenges)
            .map(|(_, mut point)| {
                let mut iopp_round_query = Vec::with_capacity(iopp_oracles.len() + 1);

                // Merkle queries over F
                let oracle_rhs_start = commitment.tree.size() >> 1;
                let sibling_point = point ^ oracle_rhs_start;
                let left = std::cmp::min(point, sibling_point);
                let right = oracle_rhs_start + left;
                point = left;

                let first_round_query = BasefoldIOPPQuerySingleRound {
                    left: commitment.tree.index_query(left),
                    right: commitment.tree.index_query(right),
                };

                first_round_queries.push(first_round_query);

                // Merkle queries over ExtF
                iopp_oracles.iter().for_each(|oracle| {
                    // NOTE: since the oracle length is always a power of 2,
                    // so the right hand side starts from directly div by 2.
                    let oracle_rhs_start = oracle.size() >> 1;

                    // NOTE: dirty trick, oracle rhs starting index is a pow of 2.
                    // now that we want to find a sibling point w.r.t the index,
                    // by plus (or minus) against point, so xor should work.
                    let sibling_point = point ^ oracle_rhs_start;

                    let left = std::cmp::min(point, sibling_point);
                    let right = oracle_rhs_start + left;

                    // NOTE: update point for next round of IOPP querying
                    point = left;

                    iopp_round_query.push(BasefoldIOPPQuerySingleRound {
                        left: oracle.index_query(left),
                        right: oracle.index_query(right),
                    })
                });

                // todo: include first round query in the iopp round query
                BasefoldIOPPQuery { iopp_round_query }
            })
            .collect::<Vec<_>>();
        end_timer!(timer);

        BasefoldProof {
            sumcheck_transcript: SumcheckInstanceProof::new(sumcheck_polys),
            iopp_oracles: iopp_oracles.iter().map(|t| t.root()).collect(),
            iopp_last_oracle_message,
            first_iopp_query: first_round_queries,
            randomness: rs,
            iopp_queries: rest_iopp_queries,
        }
    }

    fn verify(
        verifier_param: &Self::VerifierParam,
        commitment: &Self::Commitment,
        opening_point: &Self::Point,
        value: &Self::Evaluation,
        proof: &Self::Proof,
        transcript: &mut Self::Transcript,
    ) -> bool {
        let num_vars = opening_point.len();

        // smh -- endianess hell strikes again
        let mut opening_point = opening_point.clone();
        opening_point.reverse();

        let value_lifted = ExtF::from(*value);
        let opening_point_lifted: Vec<ExtF> =
            opening_point.iter().map(|x| ExtF::from(*x)).collect();

        // NOTE: check sumcheck statement:
        // f(z) = \sum_{r \in {0, 1}^n} (f(r) \eq(r, z)) can be reduced to
        // f_r_eq_zr = f(rs) \eq(rs, z)
        let (f_r_eq_zr, rs) =
            proof
                .sumcheck_transcript
                .verify(*value, num_vars, MERGE_POLY_DEG, transcript);

        println!("verifier f(z): {:?}", value);
        println!("verifier f(r) * eq(z,r): {:?}", f_r_eq_zr);

        let eq_zr = EqPolynomial::eq_vec(&opening_point, &rs);
        // EqPolynomial::new(opening_point_lifted).evaluate(&rs);
        println!("verifier eq(z, r): {:?}", eq_zr);

        let f_r = f_r_eq_zr * eq_zr.inv().unwrap();

        println!("verifier f(r): {:?}", f_r);

        // NOTE: Basefold IOPP fold each round with rs (backwards),
        // so the last round of RS code should be all f(rs).
        if proof.iopp_last_oracle_message.len() != 1 << verifier_param.rate_bits {
            return false;
        }

        println!(
            "iopp_last_oracle_message: {:?}",
            proof.iopp_last_oracle_message
        );

        // this check fails
        if proof
            .iopp_last_oracle_message
            .iter()
            .any(|&x| x.data != f_r)
        {
            return false;
        }

        let commitment_root = commitment.tree.root();
        let oracles = std::iter::once(&commitment_root)
            .chain(proof.iopp_oracles.iter())
            .cloned()
            .take(num_vars)
            .collect::<Vec<_>>();

        let points = verifier_param.iopp_challenges(num_vars, transcript);

        if !proof
            .iopp_queries
            .iter()
            .enumerate()
            .all(|(i, iopp_query)| iopp_query.verify(verifier_param, points[i], &oracles, &rs))
        {
            return false;
        }

        true
    }
}
