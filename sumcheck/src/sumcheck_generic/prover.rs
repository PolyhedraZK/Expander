use arith::Field;
use polynomials::{MultiLinearPoly, MultilinearExtension};

use super::{IOPProverMessage, IOPProverState};

impl<F: Field> IOPProverState<F> {
    /// Initialize the prover state to argue for the sum of the input polynomial
    /// over {0,1}^`num_vars`.
    pub fn prover_init(polynomials: &[MultiLinearPoly<F>]) -> Self {
        Self {
            challenges: Vec::with_capacity(polynomials[0].num_vars()),
            round: 0,
            init_num_vars: polynomials[0].num_vars(),
            mle_list: polynomials.to_vec(),
        }
    }

    /// Receive message from verifier, generate prover message, and proceed to
    /// next round.
    ///
    /// Main algorithm used is from section 3.2 of [XZZPS19](https://eprint.iacr.org/2019/317.pdf#subsection.3.2).
    pub fn prove_round_and_update_state(&mut self, challenge: &Option<F>) -> IOPProverMessage<F> {
        if self.round >= self.init_num_vars {
            panic!("prover is not active")
        }

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

        if let Some(chal) = challenge {
            if self.round == 0 {
                panic!("first round should not have a challenge");
            }
            self.challenges.push(*chal);

            let r = self.challenges[self.round - 1];

            for mle in self.mle_list.iter_mut() {
                mle.fix_top_variable(r);
            }
        } else if self.round > 0 {
            panic!("verifier message is empty")
        }

        self.round += 1;

        let len = self.mle_list[0].coeffs.len() / 2;
        let mut f_0 = F::zero();
        let mut f_1 = F::zero();

        for mle in self.mle_list.iter() {
            // evaluate the polynomial at the current point
            // and sum the evaluations for f_0 and f_1
            let coeffs = mle.coeffs.as_slice();
            f_0 += coeffs[..len].iter().sum::<F>();
            f_1 += coeffs[len..].iter().sum::<F>();
        }

        let msg = IOPProverMessage {
            evaluations: vec![f_0, f_1],
        };

        msg
    }
}
