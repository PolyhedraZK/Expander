use std::vec;

use arith::{Field, FieldSerde, VectorizedM31, M31, VECTORIZEDM31_INV_2};

use crate::{
    grind, Circuit, CircuitLayer, Config, Gate, Proof, RawCommitment, Transcript,
    _eq_evals_at_primitive,
};

// type FPrimitive = M31;
// type F = VectorizedM31;

fn degree_2_eval<F: Field>(p0: F, p1: F, p2: F, x: F::BaseField) -> F {
    let c0 = &p0;
    let c2 = F::INV_2 * (p2 - p1 - p1 + p0);
    let c1 = p1 - p0 - c2;
    *c0 + (c2 .mul_base_elem(& x) + c1) .mul_base_elem(& x)
}

fn eval_sparse_circuit_connect_poly<F: Field, const INPUT_NUM: usize>(
    gates: &[Gate<F, INPUT_NUM>],
    rz0: &[F::BaseField],
    rz1: &[F::BaseField],
    alpha: F::BaseField,
    beta: F::BaseField,
    ris: &[Vec<F::BaseField>],
) -> F::BaseField {
    let mut eq_evals_at_rz0 = vec![F::BaseField::zero(); 1 << rz0.len()];
    let mut eq_evals_at_rz1 = vec![F::BaseField::zero(); 1 << rz1.len()];

    _eq_evals_at_primitive(rz0, &alpha, &mut eq_evals_at_rz0);
    _eq_evals_at_primitive(rz1, &beta, &mut eq_evals_at_rz1);

    let mut eq_evals_at_ris = vec![vec![]; INPUT_NUM];
    for i in 0..INPUT_NUM {
        eq_evals_at_ris[i] = vec![F::BaseField::zero(); 1 << ris[i].len()];
        _eq_evals_at_primitive(&ris[i], &F::BaseField::one(), &mut eq_evals_at_ris[i])
    }

    let mut v = F::BaseField::zero();
    for g in gates {
        let mut prod = eq_evals_at_rz0[g.o_id] + eq_evals_at_rz1[g.o_id];
        for i in 0..INPUT_NUM {
            prod = prod * eq_evals_at_ris[i][g.i_ids[i]];
        }
        v += prod * g.coef;
    }
    v
}

fn sumcheck_verify_gkr_layer<F:Field+FieldSerde>(
    layer: &CircuitLayer<F>,
    rz0: &[Vec<F::BaseField>],
    rz1: &[Vec<F::BaseField>],
    claimed_v0: &[F],
    claimed_v1: &[F],
    alpha: F::BaseField,
    beta: F::BaseField,
    proof: &mut Proof,
    transcript: &mut Transcript,
    config: &Config,
) -> (
    bool,
    Vec<Vec<F::BaseField>>,
    Vec<Vec<F::BaseField>>,
    Vec<F>,
    Vec<F>,
) {
    let var_num = layer.input_var_num;
    let mut sum = (0..config.get_num_repetitions())
        .map(|i| claimed_v0[i] .mul_base_elem(&alpha) + claimed_v1[i].mul_base_elem(& beta))
        .collect::<Vec<_>>();
    let mut rx = vec![vec![]; config.get_num_repetitions()];
    let mut ry = vec![vec![]; config.get_num_repetitions()];
    let mut vx_claim = vec![F::zero(); config.get_num_repetitions()];
    let mut verified = true;
    for i_var in 0..var_num * 2 {
        for j in 0..config.get_num_repetitions() {
            let p0 = proof.get_next_and_step();
            let p1 = proof.get_next_and_step();
            let p2 = proof.get_next_and_step();
            transcript.append_f(p0);
            transcript.append_f(p1);
            transcript.append_f(p2);
            // if j == 0 {
            //     println!(
            //         "i_var={} j={} p0 p1 p2: {:?} {:?} {:?}",
            //         i_var, j, p0, p1, p2
            //     );
            // }
            let r = transcript.challenge_f::<F>();

            if i_var < var_num {
                rx[j].push(r);
            } else {
                ry[j].push(r);
            }
            verified &= (p0 + p1) == sum[j];
            // assert!(
            //     verified,
            //     "Sumcheck verification failed at i_var={}, j={}, left = {:?}, right = {:?}",
            //     i_var,
            //     j,
            //     p0 + p1,
            //     sum[j]
            // );
            sum[j] = degree_2_eval(p0, p1, p2, r);

            if i_var == var_num - 1 {
                vx_claim[j] = proof.get_next_and_step();
                sum[j] = sum[j]
                    - vx_claim[j]
                        .mul_base_elem(& eval_sparse_circuit_connect_poly(
                            &layer.add,
                            &rz0[j],
                            &rz1[j],
                            alpha,
                            beta,
                            &[rx[j].clone()],
                        ));
                transcript.append_f(vx_claim[j]);
            }
        }
    }
    let mut vy_claim:Vec<F> = vec![];
    for j in 0..config.get_num_repetitions() {
        vy_claim.push(proof.get_next_and_step());
        verified &= sum[j]
            == vx_claim[j]
                * vy_claim[j]
                .mul_base_elem(& eval_sparse_circuit_connect_poly(
                    &layer.mul,
                    &rz0[j],
                    &rz1[j],
                    alpha,
                    beta,
                    &[rx[j].clone(), ry[j].clone()],
                ));
        // assert!(verified, "Sumcheck verification failed at j = {}", j);
        transcript.append_f(vy_claim[j]);
    }
    (verified, rx, ry, vx_claim, vy_claim)
}

pub fn gkr_verify<F:Field+FieldSerde>(
    circuit: &Circuit<F>,
    claimed_v: &[F],
    transcript: &mut Transcript,
    proof: &mut Proof,
    config: &Config,
) -> (
    bool,
    Vec<Vec<F::BaseField>>,
    Vec<Vec<F::BaseField>>,
    Vec<F>,
    Vec<F>,
) {
    let layer_num = circuit.layers.len();
    let mut rz0 = vec![vec![]; config.get_num_repetitions()];
    let mut rz1 = vec![vec![]; config.get_num_repetitions()];
    for _ in 0..circuit.layers.last().unwrap().output_var_num {
        for j in 0..config.get_num_repetitions() {
            rz0[j].push(transcript.challenge_f::<F>());
            rz1[j].push(F::BaseField::zero());
        }
    }
    let mut alpha = F::BaseField::one();
    let mut beta = F::BaseField::zero();
    let mut claimed_v0 = claimed_v.to_vec();
    let mut claimed_v1 = vec![F::zero(); claimed_v.len()];

    let mut verified = true;
    for i in (0..layer_num).rev() {
        let cur_verified;
        (cur_verified, rz0, rz1, claimed_v0, claimed_v1) = sumcheck_verify_gkr_layer(
            &circuit.layers[i],
            &rz0,
            &rz1,
            &claimed_v0,
            &claimed_v1,
            alpha,
            beta,
            proof,
            transcript,
            config,
        );
        // assert!(cur_verified, "Sumcheck verification failed at layer {}", i);
        verified &= cur_verified;
        alpha = transcript.challenge_f::<F>();
        beta = transcript.challenge_f::<F>();
        // println!(
        //     "Layer {} verified with alpha={:?} and beta={:?}, claimed_v0={:?}, claimed_v1={:?}",
        //     i, alpha, beta, claimed_v0, claimed_v1
        // );
    }
    (verified, rz0, rz1, claimed_v0, claimed_v1)
}

pub struct Verifier {
    config: Config,
}

impl Verifier {
    pub fn new(config: &Config) -> Self {
        Verifier {
            config: config.clone(),
        }
    }

    pub fn verify<F:Field+FieldSerde>(&self, circuit: &Circuit<F>, claimed_v: &[F], proof: &Proof) -> bool {
        let poly_size = circuit.layers.first().unwrap().input_vals.evals.len();
        let commitment = RawCommitment::deserialize_from(&proof.bytes, poly_size);

        let mut transcript = Transcript::new();
        transcript.append_u8_slice(&proof.bytes, commitment.size());

        // ZZ: shall we use probabilistic grinding so the verifier can avoid this cost?
        // (and also be recursion friendly)
        grind::<F>(&mut transcript, &self.config);
        let mut proof = proof.clone(); // FIXME: consider separating pointers to make proof always immutable?
        proof.step(commitment.size() + 256 / 8);

        let (mut verified, rz0, rz1, claimed_v0, claimed_v1) = gkr_verify(
            circuit,
            claimed_v,
            &mut transcript,
            &mut proof,
            &self.config,
        );

        // assert!(verified, "GKR verification failed");

        match self.config.polynomial_commitment_type {
            crate::PolynomialCommitmentType::Raw => {
                // for Raw, no need to load from proof
                for i in 0..self.config.get_num_repetitions() {
                    println!("rz0[{}].size() = {}", i, rz0[i].len());
                    println!("Poly_vals.size() = {}", commitment.poly_vals.len());
                    verified &= commitment.verify(&rz0[i], claimed_v0[i]);
                    verified &= commitment.verify(&rz1[i], claimed_v1[i]);
                }
            }
            _ => todo!(),
        }

        verified
    }
}
