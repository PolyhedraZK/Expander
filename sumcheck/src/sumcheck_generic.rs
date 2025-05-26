//! Generic sumcheck protocol implementation.
//! Modified from Hyperplonk code base, with modifications.
// TODO: for now this only supports the case where challenge field and base field are the same.

use std::sync::Arc;

use arith::Field;
use gkr_engine::Transcript;
use polynomials::{MultiLinearPoly, MultilinearExtension};

/// Prover State
pub struct ProverState<F: Field> {
    pub challenges: Vec<F>,
    pub round: usize,
    pub mle_polys_ref: Arc<Vec<MultiLinearPoly<F>>>,
    pub num_vars: usize,
}

/// Verifier State
pub struct VerifierState<F: Field> {
    pub round: usize,
    pub num_vars: usize,
    pub max_degree: usize,
    pub finished: bool,
    /// a list storing the univariate polynomial in evaluation form sent by the
    /// prover at each round
    pub polynomials_received: Vec<Vec<F>>,
    /// a list storing the randomness sampled by the verifier at each round
    pub challenges: Vec<F>,
}

/// Proof
pub struct Proof<F: Field> {
    /// list of messages send from prover to verifier at each round
    pub iop_message: Vec<IOPProverMessage<F>>,
    /// challenge received from verifier at each round
    pub challenges: Vec<F>,
}

/// message sent from prover to verifier at each round
pub struct IOPProverMessage<F: Field> {
    /// evaluations of the polynomial at the current round
    pub evaluations: Vec<F>,
}

impl<F: Field> ProverState<F> {
    /// Initialize the prover state to argue for the sum of the input polynomial
    /// over {0,1}^`num_vars`.
    fn prover_init(polynomials: Vec<MultiLinearPoly<F>>) -> Self {
        let num_vars = polynomials[0].num_vars();
        for i in 1..polynomials.len() {
            assert_eq!(num_vars, polynomials[i].num_vars());
        }

        Self {
            challenges: Vec::with_capacity(num_vars),
            round: 0,
            mle_polys_ref: Arc::new(polynomials),
            num_vars,
        }
    }

    /// Receive message from verifier, generate prover message, and proceed to
    /// next round.
    ///
    /// Main algorithm used is from section 3.2 of [XZZPS19](https://eprint.iacr.org/2019/317.pdf#subsection.3.2).
    fn prove_round_and_update_state(&mut self, challenge: &Option<F>) -> IOPProverMessage<F> {
        if self.round >= self.num_vars {
            panic!("Prover has already finished all rounds");
        }

        // let fix_argument = start_timer!(|| "fix argument");

        // Step 1:
        // fix argument and evaluate f(x) over x_m = r; where r is the challenge
        // for the current round, and m is the round number, indexed from 1
        //
        // i.e.:
        // at round m <= n, for each mle g(x_1, ... x_n) within the flattened_mle
        // which has already been evaluated to
        //
        //    g(r_1, ..., r_{m-1}, x_m ... x_n)
        //
        // eval g over r_m, and mutate g to g(r_1, ... r_m,, x_{m+1}... x_n)

        // let mut flattened_ml_extensions: Vec<DenseMultilinearExtension<F>> = self
        //     .poly
        //     .flattened_ml_extensions
        //     .par_iter()
        //     .map(|x| x.as_ref().clone())
        //     .collect();

        if let Some(chal) = challenge {
            if self.round == 0 {
                panic!("first round should be prover first.");
            }
            self.challenges.push(*chal);

            let r = self.challenges[self.round - 1];

            self.mle_polys_ref
                .iter_mut()
                .for_each(|mle| mle.fix_variables(&[r]));
        } else if self.round > 0 {
            panic!("verifier message is empty");
        }

        self.round += 1;

        let products_list = self.poly.products.clone();
        let mut products_sum = vec![F::zero(); self.poly.aux_info.max_degree + 1];

        // Step 2: generate sum for the partial evaluated polynomial:
        // f(r_1, ... r_m,, x_{m+1}... x_n)

        products_sum.iter_mut().enumerate().for_each(|(t, e)| {
            let t = F::from(t as u32);
            let one_minus_t = F::one() - t;

            for b in 0..1 << (self.poly.aux_info.num_variables - self.round) {
                // evaluate P_round(t)
                for (coefficient, products) in products_list.iter() {
                    let num_mles = products.len();
                    let mut product = *coefficient;
                    for &f in products.iter().take(num_mles) {
                        let table = &flattened_ml_extensions[f]; // f's range is checked in init
                        product *= table[b << 1] + (table[(b << 1) + 1] - table[b << 1]) * t;
                    }
                    *e += product;
                }
            }
        });

        // update prover's state to the partial evaluated polynomial
        self.poly.flattened_ml_extensions = flattened_ml_extensions
            .par_iter()
            .map(|x| Arc::new(x.clone()))
            .collect();

        Ok(IOPProverMessage {
            evaluations: products_sum,
        })
    }
}
