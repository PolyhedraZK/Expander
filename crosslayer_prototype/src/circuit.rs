use arith::Field;
use gkr_field_config::GKRFieldConfig;
use rand::RngCore;
use crate::gates::{SimpleGateAdd, SimpleGateConst, SimpleGateMul};

use super::CrossLayerRelay;

#[derive(Debug, Clone, Default)]
pub struct GenericLayer<C: GKRFieldConfig> {
    pub layer_id: usize,
    pub layer_size: usize,
    pub input_layer_size: usize,

    pub add_gates: Vec<SimpleGateAdd<C>>,
    pub mul_gates: Vec<SimpleGateMul<C>>,
    pub const_gates: Vec<SimpleGateConst<C>>,
    pub relay_gates: Vec<CrossLayerRelay<C>>,
}

impl<C: GKRFieldConfig> GenericLayer<C> {
    pub fn random_for_testing(mut rng: impl RngCore, layer_id: usize, layer_sizes: &[usize]) -> Self {
        if layer_id == 0 {
            return Self::default();
        }
        let output_size = layer_sizes[layer_id];
        let input_size = layer_sizes[layer_id - 1];
        let mut layer = GenericLayer::<C> { layer_id, layer_size: output_size, input_layer_size: input_size, ..Default::default() };

        let n_gates = output_size * 2; // Not necessarily 2, just for testing

        for _ in 0..n_gates {
            let gate_type = rng.next_u64() as usize % 4;
            match gate_type {
                0 => {
                    let gate = SimpleGateAdd::random_for_testing(&mut rng, output_size, input_size);
                    layer.add_gates.push(gate);
                },
                1 => {
                    let gate = SimpleGateMul::random_for_testing(&mut rng, output_size, input_size);
                    layer.mul_gates.push(gate);
                },
                2 => {
                    let gate = SimpleGateConst::random_for_testing(&mut rng, output_size, input_size);
                    layer.const_gates.push(gate);
                },
                3 => {
                    let i_layer = rng.next_u64() as usize % layer_id;
                    let gate = CrossLayerRelay::random_for_testing(&mut rng, output_size, layer_sizes[i_layer], i_layer);
                    layer.relay_gates.push(gate);
                },
                _ => unreachable!(),
            }
        }

        layer
    }
}

#[derive(Debug, Clone, Default)]
pub struct CrossLayerCircuitEvals<C: GKRFieldConfig> {
    pub vals: Vec<Vec<C::ChallengeField>>,
}

#[derive(Debug, Clone, Default)]
pub struct CrossLayerCircuit<C: GKRFieldConfig> {
    pub layers: Vec<GenericLayer<C>>,
}

impl<C: GKRFieldConfig> CrossLayerCircuit<C> {
    pub fn random_for_testing(mut rng: impl RngCore, n_layers: usize) -> Self {
        let layer_sizes = (0..n_layers).map(|i_layer| 1usize << (n_layers - 1 - i_layer)).collect::<Vec<_>>();

        let mut circuit = Self::default();
        circuit.layers.push(GenericLayer::<C> {
            layer_id: 0,
            layer_size: layer_sizes[0],
            ..Default::default()
        }); // layer 0 is input layer, no gates

        for i in 1..layer_sizes.len() {
            let layer = GenericLayer::random_for_testing(&mut rng, i, &layer_sizes);
            circuit.layers.push(layer);
        }
        circuit
    }

    pub fn evaluate(&self, input: &[C::ChallengeField]) -> CrossLayerCircuitEvals<C> {
        let mut vals = Vec::with_capacity(self.layers.len());
        vals.push(input.to_vec());

        for i_layer in 1..self.layers.len() {
            let layer = &self.layers[i_layer];
            let mut new_layer_vals = vec![C::ChallengeField::zero(); layer.layer_size];

            for gate in &layer.add_gates {
                new_layer_vals[gate.o_id] += vals[i_layer - 1][gate.i_ids[0]] * gate.coef;
            }

            for gate in &layer.mul_gates {
                new_layer_vals[gate.o_id] += vals[i_layer - 1][gate.i_ids[0]] * vals[i_layer - 1][gate.i_ids[1]] * gate.coef;
            }

            for gate in &layer.const_gates {
                new_layer_vals[gate.o_id] += gate.coef;
            }

            for gate in &layer.relay_gates {
                new_layer_vals[gate.o_id] += vals[gate.i_layer][gate.i_id] * gate.coef;
            }

            vals.push(new_layer_vals);
        }

        CrossLayerCircuitEvals { vals }
    }

    pub fn max_num_input_var(&self) -> usize {
        self.layers.iter().map(|layer| 
            if layer.input_layer_size > 0 {
                layer.input_layer_size.trailing_zeros() as usize
            } else {
                0
            }
        ).max().unwrap()
    }

    pub fn max_num_output_var(&self) -> usize {
        self.layers.iter().map(|layer| 
            if layer.layer_size > 0 {
                layer.layer_size.trailing_zeros() as usize
            } else {
                0
            }
        ).max().unwrap()
    }
}

// CrossLayerConnections is a struct that stores the connections between the gates of different layers
// TODO-Optimization: This does not seem to be memory efficient
#[derive(Debug, Clone, Default)]
pub struct CrossLayerConnections {
    pub connections: Vec<Vec<Vec<(usize, usize)>>>, // connections[i][j] = (k, l) means output layer i, gate k is connected to input layer j, gate l
}

impl CrossLayerConnections {
    pub fn parse_circuit<C: GKRFieldConfig>(c: &CrossLayerCircuit<C>) -> Self {
        let mut connections = vec![vec![vec![]; c.layers.len()]; c.layers.len()];

        for o_layer in 1..c.layers.len() {
            for gate in &c.layers[o_layer].relay_gates {
                connections[o_layer][gate.i_layer].push((gate.o_id, gate.i_id));
            }
        }

        CrossLayerConnections { connections }
    }
}