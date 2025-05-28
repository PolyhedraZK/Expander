// Copyright (c) 2023 Espresso Systems (espressosys.com)
// This file is part of the HyperPlonk library.

// You should have received a copy of the MIT License
// along with the HyperPlonk library. If not, see <https://mit-license.org/>.

//! Prover subroutines for a SumCheck protocol.

use std::sync::Arc;

use arith::Field;
use polynomials::{MultiLinearPoly, MultilinearExtension, VirtualPolynomial};

use crate::batch_inversion;

use super::{IOPProverMessage, IOPProverState};

impl<F: Field> IOPProverState<F> {
    /// Initialize the prover state to argue for the sum of the input polynomial
    /// over {0,1}^`num_vars`.
    pub fn prover_init(
        // polynomials:
        // &[MultiLinearPoly<F>]
        polynomial: MultiLinearPoly<F>, // &VirtualPolynomial<F>
    ) -> Self {
        // if polynomial.aux_info.num_variables == 0 {
        //     panic!("Prover cannot prove a polynomial with no variables.");
        // }

        Self {
            challenges: Vec::with_capacity(polynomial.num_vars()),
            round: 0,
            init_num_vars: polynomial.num_vars(),
            // mle_list: polynomials.to_vec(),
            mle: polynomial.clone(),
            // extrapolation_aux: (1..polynomial.aux_info.max_degree)
            //     .map(|degree| {
            //         let points = (0..1 + degree as u32).map(F::from).collect::<Vec<_>>();
            //         let weights = barycentric_weights(&points);
            //         (points, weights)
            //     })
            //     .collect(),
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

        println!(
            "Prover round {}: fixing variable with challenge {:?}",
            self.round, challenge
        );

        let mut mle = self.mle.clone();
        println!("mle before eval: {:?}", mle);

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
        // let mut flattened_ml_extensions: Vec<MultiLinearPoly<F>> = self
        //     .poly
        //     .flattened_ml_extensions
        //     .iter()
        //     .map(|x| x.as_ref().clone())
        //     .collect();

        if let Some(chal) = challenge {
            if self.round == 0 {
                panic!("first round should not have a challenge");
            }
            self.challenges.push(*chal);

            let r = self.challenges[self.round - 1];

            mle.fix_top_variable(r);
            // mle.fix_bottom_variable(&r);

            // self.mle_list.iter_mut().for_each(|mle|
            //          // (ZZ: may be buggy)
            //          mle.fix_bottom_variable(&r));
            //         // mle.fix_top_variable(r));
        } else if self.round > 0 {
            panic!("verifier message is empty")
        }

        println!("mle after eval: {:?}", mle);

        // end_timer!(fix_argument);

        self.round += 1;

        let len = mle.coeffs.len() / 2;
        let f_0 = mle.coeffs[..len].iter().sum::<F>();
        let f_1 = mle.coeffs[len..].iter().sum::<F>();

        self.mle = mle;

        let msg = IOPProverMessage {
            evaluations: vec![f_0, f_1],
        };

        println!("Prover message: {:?}", msg);
        msg

        // let products_list = self.poly.products.clone();
        // let mut products_sum = vec![F::zero(); self.poly.aux_info.max_degree + 1];

        // println!("max degree: {:?}", self.poly.aux_info.max_degree);
        // println!("product sum: {:?}", products_sum);

        // Step 2: generate sum for the partial evaluated polynomial:
        // f(r_1, ... r_m,, x_{m+1}... x_n)

        // let mut sum = F::zero();
        // // products_sum.iter_mut().enumerate().for_each(|(t, e)| {
        //     // for b in 0..1 << (self.poly.aux_info.num_variables - self.round) {
        //     for b in 0..1 << (self.init_num_vars - self.round) {
        //         // evaluate P_round(t)
        //         for mle in self.mle_list.iter() {
        //             // let num_mles = products.len();
        //             // let mut product = *coefficient;
        //             // for &f in products.iter().take(num_mles) {
        //                 // let table = &flattened_ml_extensions[f]; // f's range is checked in
        // init                 product *= mle[b << 1] * (F::one() - F::from(t as u32))
        //                     + mle[(b << 1) + 1] * F::from(t as u32);
        //             // }
        //            sum += product;
        //         }
        //     }
        // }
        // );

        // products_list.iter().for_each(|(coefficient, products)| {
        //     let mut sum = (0..1 << (self.poly.aux_info.num_variables - self.round))
        //         .into_iter()
        //         .fold(
        //             || {
        //                 (
        //                     vec![(F::zero(), F::zero()); products.len()],
        //                     vec![F::zero(); products.len() + 1],
        //                 )
        //             },
        //             |(mut buf, mut acc), b| {
        //                 buf.iter_mut()
        //                     .zip(products.iter())
        //                     .for_each(|((eval, step), f)| {
        //                         let table = &flattened_ml_extensions[*f];
        //                         *eval = table[b << 1];
        //                         *step = table[(b << 1) + 1] - table[b << 1];
        //                     });
        //                 acc[0] += buf.iter().map(|(eval, _)| eval).product::<F>();
        //                 acc[1..].iter_mut().for_each(|acc| {
        //                     buf.iter_mut().for_each(|(eval, step)| *eval += step as &_);
        //                     *acc += buf.iter().map(|(eval, _)| eval).product::<F>();
        //                 });
        //                 (buf, acc)
        //             },
        //         )
        //         .map(|(_, partial)| partial)
        //         .reduce(
        //             || vec![F::zero(); products.len() + 1],
        //             |mut sum, partial| {
        //                 sum.iter_mut()
        //                     .zip(partial.iter())
        //                     .for_each(|(sum, partial)| *sum += partial);
        //                 sum
        //             },
        //         );
        //     sum.iter_mut().for_each(|sum| *sum *= coefficient);
        //     let extraploation = (0..self.poly.aux_info.max_degree - products.len())
        //         .into_iter()
        //         .map(|i| {
        //             let (points, weights) = &self.extrapolation_aux[products.len() - 1];
        //             let at = F::from((products.len() + 1 + i) as u32);
        //             extrapolate(points, weights, &sum, &at)
        //         })
        //         .collect::<Vec<_>>();
        //     products_sum
        //         .iter_mut()
        //         .zip(sum.iter().chain(extraploation.iter()))
        //         .for_each(|(products_sum, sum)| *products_sum += sum);
        // });

        // update prover's state to the partial evaluated polynomial
        // self.poly.flattened_ml_extensions = fl

        // todo!()
    }
}

fn barycentric_weights<F: Field>(points: &[F]) -> Vec<F> {
    let mut weights = points
        .iter()
        .enumerate()
        .map(|(j, point_j)| {
            points
                .iter()
                .enumerate()
                .filter(|&(i, _point_i)| (i != j))
                .map(|(_i, point_i)| *point_j - point_i)
                .reduce(|acc, value| acc * value)
                .unwrap_or_else(F::one)
        })
        .collect::<Vec<_>>();
    batch_inversion(&mut weights);
    weights
}

fn extrapolate<F: Field>(points: &[F], weights: &[F], evals: &[F], at: &F) -> F {
    let (coeffs, sum_inv) = {
        let mut coeffs = points.iter().map(|point| *at - point).collect::<Vec<_>>();
        batch_inversion(&mut coeffs);
        coeffs.iter_mut().zip(weights).for_each(|(coeff, weight)| {
            *coeff *= weight;
        });
        let sum_inv = coeffs.iter().sum::<F>().inv().unwrap_or_default();
        (coeffs, sum_inv)
    };
    coeffs
        .iter()
        .zip(evals)
        .map(|(coeff, eval)| *coeff * eval)
        .sum::<F>()
        * sum_inv
}
