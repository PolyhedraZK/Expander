use arith::Field;
use polynomials::SumOfProductsPoly;

use super::{IOPProverMessage, IOPProverState};

impl<F: Field> IOPProverState<F> {
    /// Initialize the prover state to argue for the sum of the input polynomial
    /// over {0,1}^`num_vars`.
    pub fn prover_init(polynomials: &SumOfProductsPoly<F>) -> Self {
        Self {
            challenges: Vec::with_capacity(polynomials.num_vars()),
            round: 0,
            init_num_vars: polynomials.num_vars(),
            mle_list: polynomials.clone(),
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

            self.mle_list.fix_top_variable(r);
        } else if self.round > 0 {
            panic!("verifier message is empty")
        }
        self.round += 1;

        let evaluations = match self.mle_list.degree() {
            2 => {
                let (h_0, h_1, h_2) = self.mle_list.extrapolate_at_0_1_2();
                vec![h_0, h_1, h_2]
            }
            3 => {
                let (h_0, h_1, h_2, h_m1) = self.mle_list.extrapolate_at_0_1_2_m1();
                vec![h_0, h_1, h_2, h_m1]
            }
            _ => {
                panic!("SumCheck protocol only supports polynomials of degree 2 or 3")
            }
        };

        println!("Prover round {}: challenge = {:?}", self.round, challenge);
        IOPProverMessage { evaluations }
    }
}
