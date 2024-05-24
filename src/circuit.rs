use arith::{Field, MultiLinearPoly};
use ark_std::test_rng;
use std::{cmp::max, fs};

#[derive(Debug, Clone)]
pub struct Gate<F: Field, const INPUT_NUM: usize> {
    pub i_ids: [usize; INPUT_NUM],
    pub o_id: usize,
    pub coef: F::BaseField,
}

pub type GateMul<F> = Gate<F, 2>;
pub type GateAdd<F> = Gate<F, 1>;

#[derive(Debug, Clone, Default)]
pub struct CircuitLayer<F: Field> {
    pub input_var_num: usize,
    pub output_var_num: usize,

    pub input_vals: MultiLinearPoly<F>,
    pub output_vals: MultiLinearPoly<F>, // empty most time, unless in the last layer

    pub mul: Vec<GateMul<F>>,
    pub add: Vec<GateAdd<F>>,
}

impl<F: Field> CircuitLayer<F> {
    pub fn evaluate(&self) -> Vec<F> {
        let mut res = vec![F::zero(); 1 << self.output_var_num];
        for gate in &self.mul {
            let i0 = &self.input_vals.evals[gate.i_ids[0]];
            let i1 = &self.input_vals.evals[gate.i_ids[1]];
            let o = &mut res[gate.o_id];
            *o += (*i0 * i1).mul_base_elem(&gate.coef);
        }
        for gate in &self.add {
            let i0 = &self.input_vals.evals[gate.i_ids[0]];
            let o = &mut res[gate.o_id];
            *o += i0.mul_base_elem(&gate.coef);
        }
        res
    }
}

#[derive(Debug, Clone, Default)]
pub struct Circuit<F: Field> {
    pub layers: Vec<CircuitLayer<F>>,
}

impl<F: Field> Circuit<F> {
    pub fn load_extracted_gates(filename_mul: &str, filename_add: &str) -> Self {
        let mut circuit = Circuit::default();
        let mul_file = fs::read_to_string(filename_mul).unwrap();
        let add_file = fs::read_to_string(filename_add).unwrap();

        let layer_num = mul_file.lines().count();
        assert_eq!(layer_num, add_file.lines().count());
        circuit.layers.resize(layer_num, CircuitLayer::default());

        for l in 0..layer_num {
            let layer = &mut circuit.layers[layer_num - l - 1]; // reversed
            let mul_input = mul_file
                .lines()
                .nth(l)
                .unwrap()
                .split(' ')
                .filter(|x| x != &"")
                .map(|x| x.parse::<usize>().unwrap())
                .collect::<Vec<_>>();
            let mul_gate_num = mul_input[0];
            assert_eq!(mul_gate_num * 4 + 1, mul_input.len());
            layer.mul = Vec::with_capacity(mul_gate_num);
            for i in 0..mul_gate_num {
                let gate = GateMul {
                    i_ids: [mul_input[i * 4 + 1], mul_input[i * 4 + 2]],
                    o_id: mul_input[i * 4 + 3],
                    coef: F::BaseField::from(mul_input[i * 4 + 4] as u32),
                };
                layer.mul.push(gate);
            }
            let add_input = add_file
                .lines()
                .nth(l)
                .unwrap()
                .split(' ')
                .filter(|x| x != &"")
                .map(|x| x.parse::<usize>().unwrap())
                .collect::<Vec<_>>();
            let add_gate_num = add_input[0];
            assert_eq!(add_gate_num * 3 + 1, add_input.len());
            layer.add = Vec::with_capacity(add_gate_num);
            for i in 0..add_gate_num {
                let gate = GateAdd {
                    i_ids: [add_input[i * 3 + 1]],
                    o_id: add_input[i * 3 + 2],
                    coef: F::BaseField::from(add_input[i * 3 + 3] as u32),
                };
                layer.add.push(gate);
            }
        }
        circuit.compute_var_num();
        circuit
    }

    fn compute_var_num(&mut self) {
        for (i, layer) in self.layers.iter_mut().enumerate() {
            let max_i = max(
                layer
                    .mul
                    .iter()
                    .map(|g| max(g.i_ids[0], g.i_ids[1]))
                    .max()
                    .unwrap_or(0),
                layer.add.iter().map(|g| g.i_ids[0]).max().unwrap_or(0),
            );
            let max_o = max(
                layer.mul.iter().map(|g| g.o_id).max().unwrap_or(0),
                layer.add.iter().map(|g| g.o_id).max().unwrap_or(0),
            );
            layer.input_var_num = max_i.next_power_of_two().trailing_zeros() as usize;
            layer.output_var_num = max_o.next_power_of_two().trailing_zeros() as usize;
            layer.input_vals.var_num = layer.input_var_num;
            log::trace!(
                "layer {} input_var_num: {} output_var_num: {}",
                i,
                layer.input_var_num,
                layer.output_var_num
            );
        }
    }

    pub fn log_input_size(&self) -> usize {
        self.layers[0].input_var_num
    }

    // Build a random mock circuit with binary inputs
    pub fn set_random_bool_input_for_test(&mut self) {
        let mut rng = test_rng();
        self.layers[0].input_vals.evals = (0..(1 << self.log_input_size()))
            .map(|_| F::random_bool_unsafe(&mut rng))
            .collect();
    }

    pub fn evaluate(&mut self) {
        for i in 0..self.layers.len() - 1 {
            self.layers[i + 1].input_vals.evals = self.layers[i].evaluate();
            log::trace!("layer {} evaluated", i);
            log::trace!(
                "First ten values: {:?}",
                self.layers[i + 1]
                    .input_vals
                    .evals
                    .iter()
                    .take(10)
                    .collect::<Vec<_>>()
            );
        }
        self.layers.last_mut().unwrap().output_vals.evals = self.layers.last().unwrap().evaluate();
        log::trace!("output evaluated");
        log::trace!(
            "First ten values: {:?}",
            self.layers
                .last()
                .unwrap()
                .output_vals
                .evals
                .iter()
                .take(10)
                .collect::<Vec<_>>()
        );
    }
}
