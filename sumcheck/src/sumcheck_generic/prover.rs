use arith::Field;
use polynomials::SumOfProductsPoly;
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};

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

            self.mle_list
                .f_and_g_pairs
                .par_iter_mut()
                .for_each(|(f, g)| {
                    // fix the top variable of f and g to r
                    f.fix_top_variable(r);
                    g.fix_top_variable(r);
                });
        } else if self.round > 0 {
            panic!("verifier message is empty")
        }

        self.round += 1;

        let len = 1 << (self.mle_list.num_vars() - 1);
        let mut h_0 = F::zero();
        let mut h_1 = F::zero();
        let mut h_2 = F::zero();

        // The following commented code is a sequential version of the computation
        //
        // for (f, g) in self.mle_list.f_and_g_pairs.iter() {
        //     // evaluate the polynomial at 0, 1 and 2
        //     // and obtain f(0)g(0) and f(1)g(1) and f(2)g(2)
        //     let f_coeffs = f.coeffs.as_slice();
        //     let g_coeffs = g.coeffs.as_slice();

        //     h_0 += f_coeffs[..len].iter().sum::<F>() * g_coeffs[..len].iter().sum::<F>();
        //     h_1 += f_coeffs[len..].iter().sum::<F>() * g_coeffs[len..].iter().sum::<F>();

        //     let f_2 = f_coeffs[..len]
        //         .iter()
        //         .zip(f_coeffs[len..].iter())
        //         .map(|(a, b)| -*a + b.double())
        //         .sum::<F>();
        //     let g2 = g_coeffs[..len]
        //         .iter()
        //         .zip(g_coeffs[len..].iter())
        //         .map(|(a, b)| -*a + b.double())
        //         .sum::<F>();
        //     h_2 += f_2 * g2;
        // }

        self.mle_list
            .f_and_g_pairs
            .par_iter()
            .map(|(f, g)| {
                // evaluate the polynomial at 0, 1 and 2
                // and obtain f(0)g(0) and f(1)g(1) and f(2)g(2)

                let f_coeffs = f.coeffs.as_slice();
                let g_coeffs = g.coeffs.as_slice();

                let h_0_local = f_coeffs[..len]
                    .iter()
                    .zip(g_coeffs[..len].iter())
                    .map(|(&f, &g)| f * g)
                    .sum::<F>();

                let h_1_local = f_coeffs[len..]
                    .iter()
                    .zip(g_coeffs[len..].iter())
                    .map(|(&f, &g)| f * g)
                    .sum::<F>();

                let h_2_local = f_coeffs[..len]
                    .iter()
                    .zip(f_coeffs[len..].iter())
                    .map(|(a, b)| -*a + b.double())
                    .zip(
                        g_coeffs[..len]
                            .iter()
                            .zip(g_coeffs[len..].iter())
                            .map(|(a, b)| -*a + b.double()),
                    )
                    .map(|(a, b)| a * b)
                    .sum::<F>();

                (h_0_local, h_1_local, h_2_local)
            })
            .collect::<Vec<_>>()
            .iter()
            .for_each(|(h_0_local, h_1_local, h_2_local)| {
                h_0 += h_0_local;
                h_1 += h_1_local;
                h_2 += h_2_local;
            });

        IOPProverMessage {
            evaluations: vec![h_0, h_1, h_2],
        }
    }
}
