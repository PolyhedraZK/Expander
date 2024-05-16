use crate::{MultiLinearPoly, M31};
use std::{cmp::max, fs};

type F = M31;

#[derive(Debug, Clone)]
pub struct Gate<const INPUT_NUM: usize> {
    i_ids: [usize; INPUT_NUM],
    o_id: usize,
    coef: F,
}

pub type GateMul = Gate<2>;
pub type GateAdd = Gate<1>;

#[derive(Debug, Clone, Default)]
pub struct CircuitLayer {
    input_var_num: usize,
    output_var_num: usize,

    input_vals: MultiLinearPoly,
    output_vals: MultiLinearPoly, // empty most time, unless in the last layer

    mul: Vec<GateMul>,
    add: Vec<GateAdd>,
}

impl CircuitLayer {
    pub fn evaluate(&self) -> Vec<F> {
        let mut res = vec![F::zero(); 1 << self.output_var_num];
        for gate in &self.mul {
            let i0 = &self.input_vals.evals[gate.i_ids[0]];
            let i1 = &self.input_vals.evals[gate.i_ids[1]];
            let o = &mut res[gate.o_id];
            *o += *i0 * i1 * gate.coef;
        }
        for gate in &self.add {
            let i0 = &self.input_vals.evals[gate.i_ids[0]];
            let o = &mut res[gate.o_id];
            *o += *i0 * gate.coef;
        }
        res
    }
}

#[derive(Debug, Clone, Default)]
pub struct Circuit {
    layers: Vec<CircuitLayer>,
}

impl Circuit {
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
                    coef: F::from(mul_input[i * 4 + 4]),
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
                    coef: F::from(add_input[i * 3 + 3]),
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
            // println!(
            //     "layer {} input_var_num: {} output_var_num: {}",
            //     i, layer.input_var_num, layer.output_var_num
            // );
        }
    }
    pub fn log_input_size(&self) -> usize {
        self.layers[0].input_var_num
    }
    pub fn set_random_bool_input(&mut self) {
        self.layers[0].input_vals.evals = (0..(1 << self.log_input_size()))
            .map(|_| F::random_bool())
            .collect();
    }
    pub fn evaluate(&mut self) {
        for i in 0..self.layers.len() - 1 {
            self.layers[i + 1].input_vals.evals = self.layers[i].evaluate();
        }
        self.layers.last_mut().unwrap().output_vals.evals = self.layers.last().unwrap().evaluate();
    }
}
