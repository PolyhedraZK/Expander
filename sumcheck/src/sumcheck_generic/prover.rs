use arith::Field;
use polynomials::{MultilinearExtension, SumOfProductsPoly};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};
#[cfg(feature = "mem-profile")]
use utils::memory_profiler::{get_memory_usage_mb, vec_size_mb};
use utils::memory_profiler::MemoryProfiler;

use super::{IOPProverMessage, IOPProverState};

impl<F: Field> IOPProverState<F> {
    /// Initialize the prover state to argue for the sum of the input polynomial
    /// over {0,1}^`num_vars`.
    pub fn prover_init(polynomials: &SumOfProductsPoly<F>) -> Self {
        let mem_profiler = MemoryProfiler::new("IOPProverState::prover_init", true);

        let num_vars = polynomials.num_vars();
        let num_pairs = polynomials.f_and_g_pairs.len();

        #[cfg(feature = "mem-profile")]
        {
            let input_size: f64 = polynomials
                .f_and_g_pairs
                .iter()
                .map(|(f, g)| vec_size_mb(&f.coeffs) + vec_size_mb(&g.coeffs))
                .sum();
            eprintln!(
                "[MEM PROVER_INIT] Input polynomials: {:.2} MB ({} pairs)",
                input_size, num_pairs
            );
        }

        mem_profiler.checkpoint("before clone");

        #[cfg(feature = "mem-profile")]
        {
            eprintln!("[MEM PROVER_INIT] About to clone polynomials (THIS IS THE BIG ALLOCATION)...");
        }

        let mle_list = polynomials.clone();

        mem_profiler.checkpoint("after clone (mle_list created)");

        #[cfg(feature = "mem-profile")]
        {
            let cloned_size: f64 = mle_list
                .f_and_g_pairs
                .iter()
                .map(|(f, g)| vec_size_mb(&f.coeffs) + vec_size_mb(&g.coeffs))
                .sum();
            eprintln!(
                "[MEM PROVER_INIT] Cloned mle_list: {:.2} MB",
                cloned_size
            );
        }

        mem_profiler.checkpoint("before init_sum_of_vals");

        let init_sum_of_vals: Vec<F> = polynomials
            .f_and_g_pairs
            .par_iter()
            .map(|(f, g)| {
                f.coeffs
                    .iter()
                    .zip(g.coeffs.iter())
                    .map(|(&f, &g)| f * g)
                    .sum::<F>()
            })
            .collect();

        mem_profiler.checkpoint("after init_sum_of_vals");

        let eq_prefix = vec![F::one(); num_pairs];

        mem_profiler.end();

        Self {
            challenges: Vec::with_capacity(num_vars),
            round: 0,
            init_num_vars: num_vars,
            mle_list,
            init_sum_of_vals,
            eq_prefix,
        }
    }
    pub fn prover_init_owned(polynomials: SumOfProductsPoly<F>) -> Self {
        let mem_profiler = MemoryProfiler::new("IOPProverState::prover_init", true);

        let num_vars = polynomials.num_vars();
        let num_pairs = polynomials.f_and_g_pairs.len();

        #[cfg(feature = "mem-profile")]
        {
            let input_size: f64 = polynomials
                .f_and_g_pairs
                .iter()
                .map(|(f, g)| vec_size_mb(&f.coeffs) + vec_size_mb(&g.coeffs))
                .sum();
            eprintln!(
                "[MEM PROVER_INIT] Input polynomials: {:.2} MB ({} pairs)",
                input_size, num_pairs
            );
        }

        mem_profiler.checkpoint("before init_sum_of_vals");

        let init_sum_of_vals: Vec<F> = polynomials
            .f_and_g_pairs
            .par_iter()
            .map(|(f, g)| {
                f.coeffs
                    .iter()
                    .zip(g.coeffs.iter())
                    .map(|(&f, &g)| f * g)
                    .sum::<F>()
            })
            .collect();

        mem_profiler.checkpoint("after init_sum_of_vals");

        let eq_prefix = vec![F::one(); num_pairs];

        mem_profiler.end();

        Self {
            challenges: Vec::with_capacity(num_vars),
            round: 0,
            init_num_vars: num_vars,
            mle_list: polynomials,
            init_sum_of_vals,
            eq_prefix,
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

            self.fix_top_variable_for_poly_pairs(&r);
        } else if self.round > 0 {
            panic!("verifier message is empty")
        }

        self.round += 1;

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

        #[cfg(feature = "mem-profile")]
        let mem_before_collect = get_memory_usage_mb();

        let intermediate = self.mle_list
            .f_and_g_pairs
            .par_iter()
            .enumerate()
            .map(|(i, (f, g))| {
                // evaluate the polynomial at 0, 1 and 2
                // and obtain f(0)g(0) and f(1)g(1) and f(2)g(2)

                if let Some(sub_idx) =
                    Self::get_sub_idx(self.init_num_vars, self.round, f.num_vars())
                {
                    let len = 1 << (f.num_vars() - sub_idx - 1);
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

                    let eq_prefix_i = self.eq_prefix[i].square();
                    (
                        h_0_local * eq_prefix_i,
                        h_1_local * eq_prefix_i,
                        h_2_local * eq_prefix_i,
                    )
                } else {
                    let h = self.eq_prefix[i].square() * self.init_sum_of_vals[i];
                    (h, F::zero(), h)
                }
            })
            .collect::<Vec<_>>();

        #[cfg(feature = "mem-profile")]
        {
            let mem_after_collect = get_memory_usage_mb();
            let intermediate_size_mb = (intermediate.len() * std::mem::size_of::<(F, F, F)>()) as f64 / (1024.0 * 1024.0);
            // 只在第一轮或内存变化较大时打印，避免刷屏
            if self.round == 1 || (mem_after_collect - mem_before_collect).abs() > 10.0 {
                eprintln!(
                    "[MEM PEAK] prove_round {} | before_collect: {:.2} MB | after_collect: {:.2} MB | delta: {:+.2} MB | intermediate_vec: {:.4} MB ({} elements)",
                    self.round,
                    mem_before_collect,
                    mem_after_collect,
                    mem_after_collect - mem_before_collect,
                    intermediate_size_mb,
                    intermediate.len()
                );
            }
        }

        intermediate.iter().for_each(|(h_0_local, h_1_local, h_2_local)| {
            h_0 += h_0_local;
            h_1 += h_1_local;
            h_2 += h_2_local;
        });

        // intermediate 在这里被 drop

        IOPProverMessage {
            evaluations: vec![h_0, h_1, h_2],
        }
    }

    fn get_sub_idx(init_num_vars: usize, round: usize, local_num_vars: usize) -> Option<usize> {
        if round < init_num_vars - local_num_vars + 1 {
            None
        } else {
            Some(round - (init_num_vars - local_num_vars + 1))
        }
    }

    fn fix_top_variable_for_poly_pairs(&mut self, challenge: &F) {
        self.mle_list
            .f_and_g_pairs
            .par_iter_mut()
            .zip(self.eq_prefix.par_iter_mut())
            .for_each(|((f, g), eq_prefix)| {
                if let Some(_sub_idx) =
                    Self::get_sub_idx(self.init_num_vars, self.round, f.num_vars())
                {
                    // fix the top variable for each polynomial pair
                    f.fix_top_variable(*challenge);
                    g.fix_top_variable(*challenge);
                } else {
                    *eq_prefix *= F::one() - *challenge; // eq(challenge, 0)
                }
            });
    }
}
