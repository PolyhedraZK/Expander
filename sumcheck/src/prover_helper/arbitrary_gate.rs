//! TODO: merge this module with sumcheck_helper

use core::panic;

use arith::{Field, FieldSerde};
use polynomials::MultiLinearPoly;
use transcript::{FiatShamirHash, Transcript};

#[derive(Debug, Clone)]
pub struct SumcheckInstanceProof<F: Field> {
    pub uni_polys: Vec<UniPoly<F>>,
}

impl<F: Field + FieldSerde> SumcheckInstanceProof<F> {
    pub fn new(uni_polys: Vec<UniPoly<F>>) -> SumcheckInstanceProof<F> {
        SumcheckInstanceProof { uni_polys }
    }

    /// Create a sumcheck proof for polynomial(s) of arbitrary degree.
    ///
    /// Params
    /// - `claim`: Claimed sumcheck evaluation (note: currently unused)
    /// - `num_rounds`: Number of rounds of sumcheck, or number of variables to bind
    /// - `polys`: Dense polynomials to combine and sumcheck
    /// - `comb_func`: Function used to combine each polynomial evaluation
    /// - `transcript`: Fiat-shamir transcript
    ///
    /// Returns (SumcheckInstanceProof, r_eval_point, final_evals)
    /// - `r_eval_point`: Final random point of evaluation
    /// - `final_evals`: Each of the polys evaluated at `r_eval_point`
    pub fn prove_arbitrary<Func, H, T>(
        _claim: &F,
        num_rounds: usize,
        polys: &mut Vec<MultiLinearPoly<F>>,
        comb_func: Func,
        combined_degree: usize,
        transcript: &mut T,
    ) -> (Self, Vec<F>, Vec<F>)
    where
        Func: Fn(&[F]) -> F + std::marker::Sync,
        H: FiatShamirHash,
        T: Transcript<H>,
    {
        let mut r: Vec<F> = Vec::new();
        let mut uni_polys: Vec<UniPoly<F>> = Vec::new();

        for _round in 0..num_rounds {
            let mle_half = polys[0].coeffs.len() / 2;

            let accum: Vec<Vec<F>> = (0..mle_half)
                .into_iter()
                .map(|poly_term_i| {
                    let mut accum = vec![F::zero(); combined_degree + 1];
                    // Evaluate P({0, ..., |g(r)|})

                    // TODO(#28): Optimize
                    // Tricks can be used here for low order bits {0,1} but general premise is a running sum for each
                    // of the m terms in the Dense multilinear polynomials. Formula is:
                    // half = | D_{n-1} | / 2
                    // D_n(index, r) = D_{n-1}[half + index] + r * (D_{n-1}[half + index] - D_{n-1}[index])

                    // eval 0: bound_func is A(low)
                    let params_zero: Vec<F> =
                        polys.iter().map(|poly| poly.coeffs[poly_term_i]).collect();
                    accum[0] += comb_func(&params_zero);

                    // TODO(#28): Can be computed from prev_round_claim - eval_point_0
                    let params_one: Vec<F> = polys
                        .iter()
                        .map(|poly| poly.coeffs[mle_half + poly_term_i])
                        .collect();
                    accum[1] += comb_func(&params_one);

                    // D_n(index, r) = D_{n-1}[half + index] + r * (D_{n-1}[half + index] - D_{n-1}[index])
                    // D_n(index, 0) = D_{n-1}[LOW]
                    // D_n(index, 1) = D_{n-1}[HIGH]
                    // D_n(index, 2) = D_{n-1}[HIGH] + (D_{n-1}[HIGH] - D_{n-1}[LOW])
                    // D_n(index, 3) = D_{n-1}[HIGH] + (D_{n-1}[HIGH] - D_{n-1}[LOW]) + (D_{n-1}[HIGH] - D_{n-1}[LOW])
                    // ...
                    let mut existing_term = params_one;
                    for eval_i in 2..(combined_degree + 1) {
                        let mut poly_evals = vec![F::zero(); polys.len()];
                        for poly_i in 0..polys.len() {
                            let poly = &polys[poly_i];
                            poly_evals[poly_i] = existing_term[poly_i]
                                + poly.coeffs[mle_half + poly_term_i]
                                - poly.coeffs[poly_term_i];
                        }

                        accum[eval_i] += comb_func(&poly_evals);
                        existing_term = poly_evals;
                    }
                    accum
                })
                .collect();

            // Vector storing evaluations of combined polynomials g(x) = P_0(x) * ... P_{num_polys} (x)
            // for points {0, ..., |g(x)|}
            let uni_poly_eval_points: Vec<F> = (0..combined_degree + 1)
                .into_iter()
                .map(|poly_i| accum.iter().map(|mle| mle[poly_i]).sum())
                .collect();

            let round_uni_poly = UniPoly::from_evals(&uni_poly_eval_points);

            // append the prover's message to the transcript
            // round_uni_poly.
            // transcript.append_field_element(f);
            println!("round_uni_poly: {:?}", round_uni_poly.coeffs.len());
            round_uni_poly
                .coeffs
                .iter()
                .for_each(|f| transcript.append_field_element(f));

            let r_j = transcript.generate_challenge();
            r.push(r_j);

            // bound all tables to the verifier's challenge
            polys
                .iter_mut()
                .for_each(|poly| poly.fix_top_variable(&r_j));
            uni_polys.push(round_uni_poly);
        }

        let final_evals = polys.iter().map(|poly| poly.coeffs[0]).collect();

        (SumcheckInstanceProof { uni_polys }, r, final_evals)
    }

    /// Verify this sumcheck proof.
    /// Note: Verification does not execute the final check of sumcheck protocol: g_v(r_v) = oracle_g(r),
    /// as the oracle is not passed in. Expected that the caller will implement.
    ///
    /// Params
    /// - `claim`: Claimed evaluation
    /// - `num_rounds`: Number of rounds of sumcheck, or number of variables to bind
    /// - `degree_bound`: Maximum allowed degree of the combined univariate polynomial
    /// - `transcript`: Fiat-shamir transcript
    ///
    /// Returns (e, r)
    /// - `e`: Claimed evaluation at random point
    /// - `r`: Evaluation point
    pub fn verify<H, T>(
        &self,
        claim: F,
        num_rounds: usize,
        degree_bound: usize,
        transcript: &mut T,
    ) -> (F, Vec<F>)
    where
        H: FiatShamirHash,
        T: Transcript<H>,
    {
        let mut e = claim;
        let mut r: Vec<F> = Vec::new();

        // verify that there is a univariate polynomial for each round
        assert_eq!(self.uni_polys.len(), num_rounds);
        for i in 0..self.uni_polys.len() {
            let poly = &self.uni_polys[i];

            // verify degree bound
            if poly.degree() != degree_bound {
                panic!("poly degree incorrect")
            }

            // check if G_k(0) + G_k(1) = e

            let poly_at_zero = poly.coeffs[0];
            let poly_at_one = poly.coeffs.iter().sum::<F>();

            assert_eq!(poly_at_zero + poly_at_one, e, "sum is not equal to e");

            // append the prover's message to the transcript
            poly.coeffs
                .iter()
                .for_each(|f| transcript.append_field_element(f));

            //derive the verifier's challenge for the next round
            let r_i = transcript.generate_challenge();

            r.push(r_i);

            // evaluate the claimed degree-ell polynomial at r_i
            e = poly.evaluate(&r_i);
        }

        (e, r)
    }
}

#[cfg(test)]
mod test {}
