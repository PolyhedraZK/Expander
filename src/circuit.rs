use arith::{Field, FieldSerde, MultiLinearPoly};
use ark_std::test_rng;
use std::{
    cmp::max,
    collections::HashMap,
    fs,
    io::{Cursor, Read},
};

use crate::Transcript;

#[derive(Debug, Clone)]
pub struct Gate<F: Field, const INPUT_NUM: usize> {
    pub i_ids: [usize; INPUT_NUM],
    pub o_id: usize,
    pub coef: F::BaseField,
}

pub type GateMul<F> = Gate<F, 2>;
pub type GateAdd<F> = Gate<F, 1>;
pub type GateConst<F> = Gate<F, 0>;

#[derive(Debug, Clone, Default)]
pub struct CircuitLayer<F: Field> {
    pub input_var_num: usize,
    pub output_var_num: usize,

    pub input_vals: MultiLinearPoly<F>,
    pub output_vals: MultiLinearPoly<F>, // empty most time, unless in the last layer

    pub mul: Vec<GateMul<F>>,
    pub add: Vec<GateAdd<F>>,
    pub const_: Vec<GateConst<F>>,
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
        for gate in &self.const_ {
            let o = &mut res[gate.o_id];
            *o += F::one().mul_base_elem(&gate.coef); // FIXME LATER: add a packing function to the trait
        }
        res
    }
}

#[derive(Debug, Clone, Default)]
pub struct Circuit<F: Field> {
    pub layers: Vec<CircuitLayer<F>>,
}

impl<F: Field> Circuit<F> {
    pub fn load_circuit(filename: &str) -> Self {
        let rc = RecursiveCircuit::<F>::load(filename);
        rc.flatten()
    }
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
            .map(|_| F::random_bool(&mut rng))
            .collect();
    }

    pub fn evaluate(&mut self) {
        for i in 0..self.layers.len() - 1 {
            self.layers[i + 1].input_vals.evals = self.layers[i].evaluate();
            log::trace!(
                "layer {} evaluated - First 10 values: {:?}",
                i,
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

// recursive format used in compiler
pub type SegmentId = usize;

pub struct Allocation {
    pub i_offset: usize,
    pub o_offset: usize,
}

pub struct Segment<F: Field> {
    pub i_var_num: usize,
    pub o_var_num: usize,
    pub child_segs: Vec<(SegmentId, Vec<Allocation>)>,
    pub gate_muls: Vec<GateMul<F>>,
    pub gate_adds: Vec<GateAdd<F>>,
    pub gate_consts: Vec<GateConst<F>>,
}

impl<F: Field + FieldSerde> Circuit<F> {
    pub fn load_witness_file(&mut self, filename: &str) {
        // note that, for data parallel, one should load multiple witnesses into different slot in the vectorized F
        let file_bytes = fs::read(filename).unwrap();
        self.load_witness_bytes(&file_bytes);
    }
    pub fn load_witness_bytes(&mut self, file_bytes: &[u8]) {
        log::trace!("witness file size: {} bytes", file_bytes.len());
        log::trace!("expecting: {} bytes", 32 * (1 << self.log_input_size()));

        let mut cursor = Cursor::new(file_bytes);
        self.layers[0].input_vals.evals = (0..(1 << self.log_input_size()))
            .map(|_| F::deserialize_from_ecc_format(&mut cursor))
            .collect();
    }
}

impl<F: Field> Segment<F> {
    pub(crate) fn contain_gates(&self) -> bool {
        !self.gate_muls.is_empty() || !self.gate_adds.is_empty() || !self.gate_consts.is_empty()
    }

    pub(crate) fn read<R: Read>(mut reader: R) -> Self {
        let i_len = u64::deserialize_from(&mut reader) as usize;
        let o_len = u64::deserialize_from(&mut reader) as usize;
        assert!(i_len.is_power_of_two());
        assert!(o_len.is_power_of_two());

        let mut ret = Segment::<F> {
            i_var_num: i_len.trailing_zeros() as usize,
            o_var_num: o_len.trailing_zeros() as usize,
            child_segs: Vec::new(),
            gate_muls: Vec::new(),
            gate_adds: Vec::new(),
            gate_consts: Vec::new(),
        };

        let child_segs_num = u64::deserialize_from(&mut reader) as usize;

        for _ in 0..child_segs_num {
            let child_seg_id = u64::deserialize_from(&mut reader) as SegmentId;

            let allocation_num = u64::deserialize_from(&mut reader) as usize;

            for _ in 0..allocation_num {
                let i_offset = u64::deserialize_from(&mut reader) as usize;
                let o_offset = u64::deserialize_from(&mut reader) as usize;
                ret.child_segs
                    .push((child_seg_id, vec![Allocation { i_offset, o_offset }]));
            }
        }

        let gate_muls_num = u64::deserialize_from(&mut reader) as usize;
        for _ in 0..gate_muls_num {
            let gate = GateMul {
                i_ids: [
                    u64::deserialize_from(&mut reader) as usize,
                    u64::deserialize_from(&mut reader) as usize,
                ],
                o_id: u64::deserialize_from(&mut reader) as usize,
                coef: F::BaseField::deserialize_from_ecc_format(&mut reader),
            };
            ret.gate_muls.push(gate);
        }

        let gate_adds_num = u64::deserialize_from(&mut reader) as usize;
        for _ in 0..gate_adds_num {
            let gate = GateAdd {
                i_ids: [u64::deserialize_from(&mut reader) as usize],
                o_id: u64::deserialize_from(&mut reader) as usize,

                coef: F::BaseField::deserialize_from_ecc_format(&mut reader),
            };
            ret.gate_adds.push(gate);
        }
        let gate_consts_num = u64::deserialize_from(&mut reader) as usize;

        log::trace!(
            "gate nums: {} mul, {} add, {} const",
            gate_muls_num,
            gate_adds_num,
            gate_consts_num
        );

        for _ in 0..gate_consts_num {
            let gate = GateConst {
                i_ids: [],
                o_id: u64::deserialize_from(&mut reader) as usize,

                coef: F::BaseField::deserialize_from_ecc_format(&mut reader),
            };
            ret.gate_consts.push(gate);
        }
        let rand_coef_idx_num = u64::deserialize_from(&mut reader) as usize;

        let mut t = Transcript::new(); // FIXME LATER: use an empty transcript to align the randomness
        for _ in 0..rand_coef_idx_num {
            let idx = u64::deserialize_from(&mut reader) as usize;

            let rand_coef = t.challenge_f::<F>();
            if idx < ret.gate_muls.len() {
                ret.gate_muls[idx].coef = rand_coef;
            } else if idx < ret.gate_muls.len() + ret.gate_adds.len() {
                ret.gate_adds[idx - ret.gate_muls.len()].coef = rand_coef;
            } else {
                ret.gate_consts[idx - ret.gate_muls.len() - ret.gate_adds.len()].coef = rand_coef;
            }
        }
        ret
    }

    pub fn scan_leaf_segments(
        &self,
        rc: &RecursiveCircuit<F>,
        cur_id: SegmentId,
    ) -> HashMap<SegmentId, Vec<Allocation>> {
        let mut ret = HashMap::new();
        if self.contain_gates() {
            ret.insert(
                cur_id,
                vec![Allocation {
                    i_offset: 0,
                    o_offset: 0,
                }],
            );
        }
        for (child_seg_id, child_allocs) in &self.child_segs {
            let leaves = rc.segments[*child_seg_id].scan_leaf_segments(rc, *child_seg_id);
            for (leaf_seg_id, leaf_allocs) in leaves {
                ret.entry(leaf_seg_id).or_insert_with(Vec::new);
                for child_alloc in child_allocs {
                    for leaf_alloc in &leaf_allocs {
                        ret.get_mut(&leaf_seg_id).unwrap().push(Allocation {
                            i_offset: child_alloc.i_offset + leaf_alloc.i_offset,
                            o_offset: child_alloc.o_offset + leaf_alloc.o_offset,
                        });
                    }
                }
            }
        }
        ret
    }
}

pub struct RecursiveCircuit<F: Field> {
    pub segments: Vec<Segment<F>>,
    pub layers: Vec<SegmentId>,
}

const MAGIC_NUM: u64 = 3626604230490605891; // b'CIRCUIT2'

impl<F: Field> RecursiveCircuit<F> {
    pub fn load(filename: &str) -> Self {
        let mut ret = RecursiveCircuit::<F> {
            segments: Vec::new(),
            layers: Vec::new(),
        };
        let file_bytes = fs::read(filename).unwrap();
        let mut cursor = Cursor::new(file_bytes);

        let magic_num = u64::deserialize_from(&mut cursor);
        assert_eq!(magic_num, MAGIC_NUM);

        let segment_num = u64::deserialize_from(&mut cursor);
        for _ in 0..segment_num {
            let seg = Segment::<F>::read(&mut cursor);
            ret.segments.push(seg);
        }

        let layer_num = u64::deserialize_from(&mut cursor);
        for _ in 0..layer_num {
            let layer_id = u64::deserialize_from(&mut cursor) as SegmentId;

            ret.layers.push(layer_id);
        }
        // TODO: configure sentinel (currently it is manually handled as sentinel is unknown before loading)
        // assert_eq!(file_bytes.len(), cur + 32);
        ret
    }
    pub fn flatten(&self) -> Circuit<F> {
        let mut ret = Circuit::default();
        // layer-by-layer conversion
        for layer_id in &self.layers {
            let layer_seg = &self.segments[*layer_id];
            let leaves = layer_seg.scan_leaf_segments(self, *layer_id);
            let mut ret_layer = CircuitLayer {
                input_var_num: layer_seg.i_var_num,
                output_var_num: layer_seg.o_var_num,
                input_vals: MultiLinearPoly::<F> {
                    var_num: layer_seg.i_var_num,
                    evals: vec![],
                },
                output_vals: MultiLinearPoly::<F> {
                    var_num: layer_seg.o_var_num,
                    evals: vec![],
                },
                mul: vec![],
                add: vec![],
                const_: vec![],
            };
            for (leaf_seg_id, leaf_allocs) in leaves {
                let leaf_seg = &self.segments[leaf_seg_id];
                for alloc in leaf_allocs {
                    for gate in &leaf_seg.gate_muls {
                        let mut gate = gate.clone();
                        gate.i_ids[0] += alloc.i_offset;
                        gate.i_ids[1] += alloc.i_offset;
                        gate.o_id += alloc.o_offset;
                        ret_layer.mul.push(gate);
                    }
                    for gate in &leaf_seg.gate_adds {
                        let mut gate = gate.clone();
                        gate.i_ids[0] += alloc.i_offset;
                        gate.o_id += alloc.o_offset;
                        ret_layer.add.push(gate);
                    }
                    for gate in &leaf_seg.gate_consts {
                        let mut gate = gate.clone();
                        gate.o_id += alloc.o_offset;
                        ret_layer.const_.push(gate);
                    }
                }
            }
            // debug print layer
            log::trace!(
                "layer {} mul: {} add: {} const:{} i_var_num: {} o_var_num: {}",
                ret.layers.len(),
                ret_layer.mul.len(),
                ret_layer.add.len(),
                ret_layer.const_.len(),
                ret_layer.input_var_num,
                ret_layer.output_var_num,
            );
            ret.layers.push(ret_layer);
        }

        ret
    }
}
