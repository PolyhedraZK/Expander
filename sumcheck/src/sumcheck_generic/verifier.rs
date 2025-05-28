use arith::Field;
use gkr_engine::Transcript;

use super::{IOPProverMessage, IOPVerifierState, SumCheckSubClaim};

impl<F: Field> IOPVerifierState<F> {
    /// Initialize the verifier's state.
    pub fn verifier_init(num_vars: usize) -> Self {
        let res = Self {
            round: 1,
            num_vars,
            finished: false,
            polynomials_received: Vec::with_capacity(num_vars),
            challenges: Vec::with_capacity(num_vars),
        };

        res
    }

    /// Run verifier for the current round, given a prover message.
    ///
    /// Note that `verify_round_and_update_state` only samples and stores
    /// challenges; and update the verifier's state accordingly. The actual
    /// verifications are deferred (in batch) to `check_and_generate_subclaim`
    /// at the last step.
    pub fn verify_round_and_update_state(
        &mut self,
        prover_msg: &IOPProverMessage<F>,
        transcript: &mut impl Transcript,
    ) -> F {
        if self.finished {
            panic!("Incorrect verifier state: Verifier is already finished.")
        }

        // In an interactive protocol, the verifier should
        //
        // 1. check if the received 'P(0) + P(1) = expected`.
        // 2. set `expected` to P(r)`
        //
        // When we turn the protocol to a non-interactive one, it is sufficient to defer
        // such checks to `check_and_generate_subclaim` after the last round.

        let challenge = transcript.generate_field_element::<F>();
        self.challenges.push(challenge);
        self.polynomials_received
            .push(prover_msg.evaluations.clone());

        if self.round == self.num_vars {
            // accept and close
            self.finished = true;
        } else {
            // proceed to the next round
            self.round += 1;
        }

        challenge
    }

    /// This function verifies the deferred checks in the interactive version of
    /// the protocol; and generate the subclaim. Returns an error if the
    /// proof failed to verify.
    ///
    /// If the asserted sum is correct, then the multilinear polynomial
    /// evaluated at `subclaim.point` will be `subclaim.expected_evaluation`.
    /// Otherwise, it is highly unlikely that those two will be equal.
    /// Larger field size guarantees smaller soundness error.
    pub fn check_and_generate_subclaim(&self, asserted_sum: &F) -> SumCheckSubClaim<F> {
        if !self.finished {
            panic!("Incorrect verifier state: Verifier has not finished.");
        }

        if self.polynomials_received.len() != self.num_vars {
            panic!("insufficient rounds")
        }

        let mut expected = *asserted_sum;

        for i in 0..self.num_vars {
            let evals = &self.polynomials_received[i];

            // the univariate polynomial f is received in its extrapolated form, i.e.,
            // f(0) = evals[0] and f(1) = evals[1].
            // now we want to compute f(r) for the challenge r = self.challenges[i]
            // that is
            // f(0) + (f(1) - f(0)) * r
            expected = evals[0] + (evals[1] - evals[0]) * self.challenges[i];
        }

        SumCheckSubClaim {
            point: self.challenges.clone(),
            // the last expected value (not checked within this function) will be included in the
            // subclaim
            expected_evaluation: expected,
        }
    }
}
