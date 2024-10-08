//! TODO: merge this module with sumcheck_helper

use arith::Field;
use transcript::{FiatShamirHash, Transcript, TranscriptInstance};

#[derive(Debug)]
pub struct SumcheckInstanceProof<F: Field> {
    pub compressed_polys: Vec<Vec<F>>,
}

impl<Field> SumcheckInstanceProof<F>{

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
pub fn prove_arbitrary<Func, F, H, T>(
    _claim: &F,
    num_rounds: usize,
    polys: &mut Vec<MultiLinearPoly<F>>,
    comb_func: Func,
    combined_degree: usize,
    transcript: &mut T,
) -> (Self, Vec<F>, Vec<F>)
where
    Func: Fn(&[F]) -> F + std::marker::Sync,
    F: Field,
    H: FiatShamirHash,
    T: Transcript<H>,
{
    let mut r: Vec<F> = Vec::new();
    let mut compressed_polys: Vec<Vec<F>> = Vec::new();

    for _round in 0..num_rounds {
        let mle_half = polys[0].len() / 2;

        let accum: Vec<Vec<F>> = (0..mle_half)
            .into_par_iter()
            .map(|poly_term_i| {
                let mut accum = vec![F::zero(); combined_degree + 1];
                // Evaluate P({0, ..., |g(r)|})

                // TODO(#28): Optimize
                // Tricks can be used here for low order bits {0,1} but general premise is a running sum for each
                // of the m terms in the Dense multilinear polynomials. Formula is:
                // half = | D_{n-1} | / 2
                // D_n(index, r) = D_{n-1}[half + index] + r * (D_{n-1}[half + index] - D_{n-1}[index])

                // eval 0: bound_func is A(low)
                let params_zero: Vec<F> = polys.iter().map(|poly| poly[poly_term_i]).collect();
                accum[0] += comb_func(&params_zero);

                // TODO(#28): Can be computed from prev_round_claim - eval_point_0
                let params_one: Vec<F> = polys
                    .iter()
                    .map(|poly| poly[mle_half + poly_term_i])
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
                        poly_evals[poly_i] = existing_term[poly_i] + poly[mle_half + poly_term_i]
                            - poly[poly_term_i];
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
            .into_par_iter()
            .map(|poly_i| accum.par_iter().map(|mle| mle[poly_i]).sum())
            .collect();

        // let round_uni_poly = UniPoly::from_evals(&eval_points);

        // append the prover's message to the transcript
        // round_uni_poly.
        // transcript.append_field_element(f);

        round_uni_poly.append_to_transcript(b"poly", transcript);
        let r_j = transcript.challenge_scalar(b"challenge_nextround");
        r.push(r_j);

        // bound all tables to the verifier's challenge
        polys
            .par_iter_mut()
            .for_each(|poly| poly.eval_top_var(&r_j));
        // compressed_polys.push(round_uni_poly.compress());
    }

    let final_evals = polys.iter().map(|poly| poly[0]).collect();

    (SumcheckInstanceProof::new(compressed_polys), r, final_evals)
}

}

