use arith::Field;
use gkr_engine::Transcript;

use super::{IOPProverMessage, IOPVerifierState, SumCheckSubClaim};

impl<F: Field> IOPVerifierState<F> {
    /// Initialize the verifier's state.
    pub fn verifier_init(num_vars: usize) -> Self {
        Self {
            round: 1,
            num_vars,
            finished: false,
            polynomials_received: Vec::with_capacity(num_vars),
            challenges: Vec::with_capacity(num_vars),
        }
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
    pub fn check_and_generate_subclaim(&self, asserted_sum: &F) -> (bool, SumCheckSubClaim<F>) {
        if !self.finished {
            panic!("Incorrect verifier state: Verifier has not finished.");
        }

        if self.polynomials_received.len() != self.num_vars {
            return (false, SumCheckSubClaim::default());
        }

        let mut expected = *asserted_sum;
        let inv_2 = F::from(2u32).inv().expect("2 is not zero in the field");
        let inv_6 = F::from(6u32).inv().expect("6 is not zero in the field");

        for i in 0..self.num_vars {
            let evals = &self.polynomials_received[i];

            // check that the sum received from last round is correct
            if expected != evals[0] + evals[1] {
                return (false, SumCheckSubClaim::default());
            }

            expected =
                interpolated_form_poly_evaluated_at_r(evals, &self.challenges[i], &inv_2, &inv_6);
        }

        (
            true,
            SumCheckSubClaim {
                point: self.challenges.clone(),
                // the last expected value (not checked within this function) will be included in
                // the subclaim
                expected_evaluation: expected,
            },
        )
    }
}

#[inline]
fn interpolated_form_poly_evaluated_at_r<F: Field>(evals: &[F], r: &F, inv_2: &F, inv_6: &F) -> F {
    match evals.len() {
        3 => {
            // the univariate polynomial f is received in its extrapolated form, i.e.,
            //   h(0) = evals[0], h(1) = evals[1], h(2) = evals[2]
            // that is, suppose h = h_0 + h_1 * x + h_2 * x^2, then
            //   h(0) = h_0
            //   h(1) = h_0 + h_1 + h_2
            //   h(2) = h_0 + 2 * h_1 + 4 * h_2
            // therefore
            //   h_0 = evals[0]
            //   h_2 = (h(2) + h(0))/2 -  h(1)
            //   h_1 = h(1) - h_0 - h_2
            let h_0 = evals[0];
            let h_2 = (evals[2] + evals[0]) * inv_2 - evals[1];
            let h_1 = evals[1] - h_0 - h_2;

            // h(r) = h_0 + h_1 * r + h_2 * r^2
            h_0 + h_1 * r + h_2 * r.square()
        }

        4 => {
            // the univariate polynomial f is received in its extrapolated form, i.e.,
            //   h(0) = evals[0], h(1) = evals[1], h(2) = evals[2], h(-1) = evals[3]
            // that is, suppose h = h_0 + h_1 * x + h_2 * x^2, then
            //   h(0) = h_0
            //   h(1) = h_0 + h_1 + h_2 + h_3
            //   h(2) = h_0 + 2 * h_1 + 4 * h_2 + 8 * h_3
            //   h(-1) = h_0 - h_1 + h_2 - h_3
            // therefore
            //   h_0 = evals[0]
            //   h_2 = (h(1) + h(-1))/2 -  h_0
            // and
            //   tmp0 := h_1 +     h_3 = (h(1) - h(-1))/2
            //   tmp1 := h_1 + 7 * h_3 = (h(2) - h(1)) - 3 * h_2
            // so
            //   h_3 = (tmp1 - tmp0) / 6
            //   h_1 = tmp0 - h_3

            let h_0 = evals[0];
            let h_2 = (evals[1] + evals[3]) * inv_2 - h_0;
            let tmp0 = (evals[1] - evals[3]) * inv_2;
            let tmp1 = (evals[2] - evals[1]) - F::from(3u32) * h_2;
            let h_3 = (tmp1 - tmp0) * inv_6;
            let h_1 = tmp0 - h_3;

            // h(r) = h_0 + h_1 * r + h_2 * r^2 + h_3 * r^3
            let r_square = r.square();
            let r_cube = r_square * r;
            h_0 + h_1 * r + h_2 * r_square + h_3 * r_cube
        }

        _ => panic!("interpolate only supports 3 or 4 evaluations"),
    }
}
