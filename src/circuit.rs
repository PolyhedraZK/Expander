use arith::{Field, FieldSerde, MultiLinearPoly};
use ark_std::{iterable::Iterable, test_rng};
use std::{
    collections::HashMap,
    fs,
    io::{Cursor, Read},
    ops::Mul,
    process::exit,
};

use crate::{GKRConfig, Transcript};

#[derive(Debug, Clone)]
pub struct Gate<C: GKRConfig, const INPUT_NUM: usize> {
    pub i_ids: [usize; INPUT_NUM],
    pub o_id: usize,
    pub coef: C::CircuitField,
    pub is_random: bool,
    pub gate_type: usize,
}

pub type GateMul<C> = Gate<C, 2>;
pub type GateAdd<C> = Gate<C, 1>;
pub type GateUni<C> = Gate<C, 1>;
pub type GateConst<C> = Gate<C, 0>;

#[derive(Debug, Clone, Default)]
pub struct CircuitLayer<C: GKRConfig> {
    pub input_var_num: usize,
    pub output_var_num: usize,

    pub input_vals: MultiLinearPoly<C::Field>,
    pub output_vals: MultiLinearPoly<C::Field>, // empty most time, unless in the last layer

    pub mul: Vec<GateMul<C>>,
    pub add: Vec<GateAdd<C>>,
    pub cst: Vec<GateConst<C>>,
    pub uni: Vec<GateUni<C>>,

    pub nb_repetition: usize,
    pub sub_layer: Option<Box<Self>>,
}

macro_rules! nb_gates_in_layer {
    ($name:ident, $gate_type:ident) => {
        pub fn $name(&self) -> usize {
            let mut n = self.$gate_type.len();
            if self.sub_layer.is_some() {
                n += self.sub_layer.as_ref().unwrap().$name();
            }
            n
        }
    };
}

impl<C: GKRConfig> CircuitLayer<C> {
    pub fn new(nb_inpt_vars: usize, nb_output_vars: usize) -> CircuitLayer<C> {
        CircuitLayer::<C> {
            input_var_num: nb_inpt_vars,
            output_var_num: nb_output_vars,

            input_vals: MultiLinearPoly::<C::Field> {
                var_num: nb_inpt_vars,
                evals: vec![],
            },
            output_vals: MultiLinearPoly::<C::Field> {
                var_num: nb_output_vars,
                evals: vec![],
            },

            mul: vec![],
            add: vec![],
            cst: vec![],
            uni: vec![],

            nb_repetition: 0,
            sub_layer: None,
        }
    }

    fn repeat_and_evaluate(
        &self,
        output: &mut Vec<C::Field>,
        input: &Vec<C::Field>,
        nb_repeat: usize,
    ) {
        let input_size = 1 << self.input_var_num;
        let output_size = 1 << self.output_var_num;

        for gate in &self.mul {
            let mut i_offset = 0;
            let mut o_offset = 0;
            for _ in 0..nb_repeat {
                let i0 = &input[gate.i_ids[0] + i_offset];
                let i1 = &input[gate.i_ids[1] + i_offset];
                let o = &mut output[gate.o_id + o_offset];
                *o += C::field_mul_circuit_field(&(*i0 * i1), &gate.coef);

                i_offset += input_size;
                o_offset += output_size;
            }
        }

        for gate in &self.add {
            let mut i_offset = 0;
            let mut o_offset = 0;
            for _ in 0..nb_repeat {
                let i0 = &input[gate.i_ids[0] + i_offset];
                let o = &mut output[gate.o_id + o_offset];
                *o += C::field_mul_circuit_field(i0, &gate.coef);

                i_offset += input_size;
                o_offset += output_size;
            }
        }

        for gate in &self.cst {
            let mut o_offset = 0;
            for _ in 0..nb_repeat {
                let o = &mut output[gate.o_id + o_offset];
                *o = C::field_add_circuit_field(o, &gate.coef);

                o_offset += output_size;
            }
        }

        for gate in &self.uni {
            let mut i_offset = 0;
            let mut o_offset = 0;
            for _ in 0..nb_repeat {
                let i0 = &input[gate.i_ids[0] + i_offset];
                let o = &mut output[gate.o_id + o_offset];
                match gate.gate_type {
                    12345 => {
                        // pow5
                        let i0_2 = i0.square();
                        let i0_4 = i0_2.square();
                        let i0_5 = i0_4 * i0;
                        *o += C::field_mul_circuit_field(&i0_5, &gate.coef);
                    }
                    12346 => {
                        // pow1
                        *o += C::field_mul_circuit_field(i0, &gate.coef);
                    }
                    _ => panic!("Unknown gate type: {}", gate.gate_type),
                }
                i_offset += input_size;
                o_offset += output_size;
            }
        }
    }

    pub fn evaluate(&self, res: &mut Vec<C::Field>) {
        res.clear();
        res.resize(1 << self.output_var_num, C::Field::zero());

        self.repeat_and_evaluate(res, &self.input_vals.evals, 1);
        if self.sub_layer.is_some() {
            self.sub_layer.as_ref().unwrap().repeat_and_evaluate(
                res,
                &self.input_vals.evals,
                self.nb_repetition,
            )
        }
    }

    pub fn identify_rnd_coefs(&mut self, rnd_coefs: &mut Vec<*mut C::CircuitField>) {
        for gate in &mut self.mul {
            if gate.is_random {
                rnd_coefs.push(&mut gate.coef);
            }
        }
        for gate in &mut self.add {
            if gate.is_random {
                rnd_coefs.push(&mut gate.coef);
            }
        }
        for gate in &mut self.cst {
            if gate.is_random {
                rnd_coefs.push(&mut gate.coef);
            }
        }
        for gate in &mut self.uni {
            if gate.is_random {
                rnd_coefs.push(&mut gate.coef);
            }
        }

        if self.sub_layer.is_some() {
            self.sub_layer
                .as_mut()
                .unwrap()
                .identify_rnd_coefs(rnd_coefs);
        }
    }

    nb_gates_in_layer!(nb_add_gates, add);
    nb_gates_in_layer!(nb_mul_gates, mul);
    nb_gates_in_layer!(nb_cst_gates, cst);
    nb_gates_in_layer!(nb_uni_gates, uni);
}

#[derive(Debug, Default)]
pub struct Circuit<C: GKRConfig> {
    pub layers: Vec<CircuitLayer<C>>,

    pub rnd_coefs_identified: bool,
    pub rnd_coefs: Vec<*mut C::CircuitField>, // unsafe
}

impl<C: GKRConfig> Clone for Circuit<C> {
    fn clone(&self) -> Circuit<C> {
        let mut ret = Circuit::<C> {
            layers: self.layers.clone(),
            rnd_coefs_identified: false,
            rnd_coefs: vec![],
        };

        if self.rnd_coefs_identified {
            ret.identify_rnd_coefs();
        }
        ret
    }
}

unsafe impl<C: GKRConfig> Send for Circuit<C> {}

macro_rules! nb_gates_in_circuit {
    ($name:ident) => {
        pub fn $name(&self) -> usize {
            self.layers.iter().map(|layer| layer.$name()).sum()
        }
    };
}

impl<C: GKRConfig> Circuit<C> {
    pub fn load_circuit(filename: &str) -> Self {
        let rc = RecursiveCircuit::<C>::load(filename);
        rc.flatten()
    }

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
            .map(|_| C::Field::deserialize_from_ecc_format(&mut cursor))
            .collect();
    }

    pub fn log_input_size(&self) -> usize {
        self.layers[0].input_var_num
    }

    // Build a random mock circuit with binary inputs
    pub fn set_random_bool_input_for_test(&mut self) {
        let mut rng = test_rng();
        self.layers[0].input_vals.evals = (0..(1 << self.log_input_size()))
            .map(|_| C::Field::random_bool(&mut rng))
            .collect();
    }

    pub fn evaluate(&mut self) {
        for i in 0..self.layers.len() - 1 {
            let (layer_p_1, layer_p_2) = self.layers.split_at_mut(i + 1);
            layer_p_1
                .last()
                .unwrap()
                .evaluate(&mut layer_p_2[0].input_vals.evals);
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
        let mut output = vec![];
        self.layers.last().unwrap().evaluate(&mut output);
        self.layers.last_mut().unwrap().output_vals.evals = output;

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

    pub fn identify_rnd_coefs(&mut self) {
        self.rnd_coefs.clear();
        for layer in &mut self.layers {
            layer.identify_rnd_coefs(&mut self.rnd_coefs);
        }
        self.rnd_coefs_identified = true;
    }

    pub fn fill_rnd_coefs(&mut self, transcript: &mut Transcript) {
        assert!(self.rnd_coefs_identified);
        for &rnd_coef_ptr in &self.rnd_coefs {
            unsafe {
                *rnd_coef_ptr = transcript.circuit_f::<C>();
            }
        }
    }

    nb_gates_in_circuit!(nb_add_gates);
    nb_gates_in_circuit!(nb_cst_gates);
    nb_gates_in_circuit!(nb_mul_gates);
    nb_gates_in_circuit!(nb_uni_gates);
}

// recursive format used in compiler
pub type SegmentId = usize;

#[derive(Debug, Clone, Copy, Default)]
pub struct Allocation {
    pub i_offset: usize,
    pub o_offset: usize,
}

pub struct Segment<C: GKRConfig> {
    pub i_var_num: usize,
    pub o_var_num: usize,
    pub child_segs: Vec<(SegmentId, Vec<Allocation>)>,
    pub gate_muls: Vec<GateMul<C>>,
    pub gate_adds: Vec<GateAdd<C>>,
    pub gate_consts: Vec<GateConst<C>>,
    pub gate_uni: Vec<GateUni<C>>,
}

impl<C: GKRConfig> Segment<C> {
    pub fn contain_gates(&self) -> bool {
        !self.gate_muls.is_empty()
            || !self.gate_adds.is_empty()
            || !self.gate_consts.is_empty()
            || !self.gate_uni.is_empty()
    }

    pub(crate) fn read<R: Read>(mut reader: R) -> Self {
        let i_len = u64::deserialize_from(&mut reader) as usize;
        let o_len = u64::deserialize_from(&mut reader) as usize;
        assert!(i_len.is_power_of_two());
        assert!(o_len.is_power_of_two());

        let mut ret = Segment::<C> {
            i_var_num: i_len.trailing_zeros() as usize,
            o_var_num: o_len.trailing_zeros() as usize,
            child_segs: Vec::new(),
            gate_muls: Vec::new(),
            gate_adds: Vec::new(),
            gate_consts: Vec::new(),
            gate_uni: Vec::new(),
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
                coef: C::CircuitField::deserialize_from_ecc_format(&mut reader),
                is_random: false,
                gate_type: 0,
            };
            ret.gate_muls.push(gate);
        }

        let gate_adds_num = u64::deserialize_from(&mut reader) as usize;
        for _ in 0..gate_adds_num {
            let gate = GateAdd {
                i_ids: [u64::deserialize_from(&mut reader) as usize],
                o_id: u64::deserialize_from(&mut reader) as usize,

                coef: C::CircuitField::deserialize_from_ecc_format(&mut reader),
                is_random: false,
                gate_type: 1,
            };
            ret.gate_adds.push(gate);
        }
        let gate_consts_num = u64::deserialize_from(&mut reader) as usize;

        for _ in 0..gate_consts_num {
            let gate = GateConst {
                i_ids: [],
                o_id: u64::deserialize_from(&mut reader) as usize,

                coef: C::CircuitField::deserialize_from_ecc_format(&mut reader),
                is_random: false,
                gate_type: 2,
            };
            ret.gate_consts.push(gate);
        }

        let gate_custom_num = u64::deserialize_from(&mut reader) as usize;
        for _ in 0..gate_custom_num {
            let gate_type = u64::deserialize_from(&mut reader) as usize;
            let in_len = u64::deserialize_from(&mut reader) as usize;
            let mut inputs = Vec::new();
            for _ in 0..in_len {
                inputs.push(u64::deserialize_from(&mut reader) as usize);
            }
            let out = u64::deserialize_from(&mut reader) as usize;
            let coef = C::CircuitField::deserialize_from_ecc_format(&mut reader);
            let gate = GateUni {
                i_ids: [inputs[0]],
                o_id: out,
                coef,
                is_random: false,
                gate_type,
            };
            ret.gate_uni.push(gate);
        }

        log::trace!(
            "gate nums: {} mul, {} add, {} const, {} custom",
            gate_muls_num,
            gate_adds_num,
            gate_consts_num,
            gate_custom_num
        );

        let rand_coef_idx_num = u64::deserialize_from(&mut reader) as usize;
        for _ in 0..rand_coef_idx_num {
            let idx = u64::deserialize_from(&mut reader) as usize;

            if idx < ret.gate_muls.len() {
                ret.gate_muls[idx].is_random = true;
            } else if idx < ret.gate_muls.len() + ret.gate_adds.len() {
                ret.gate_adds[idx - ret.gate_muls.len()].is_random = true;
            } else if idx < ret.gate_muls.len() + ret.gate_adds.len() + ret.gate_consts.len() {
                ret.gate_consts[idx - ret.gate_muls.len() - ret.gate_adds.len()].is_random = true;
            } else {
                ret.gate_uni
                    [idx - ret.gate_muls.len() - ret.gate_adds.len() - ret.gate_consts.len()]
                .is_random = true;
            }
        }
        ret
    }

    pub fn scan_leaf_segments(
        &self,
        rc: &RecursiveCircuit<C>,
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

pub struct RecursiveCircuit<C: GKRConfig> {
    pub segments: Vec<Segment<C>>,
    pub layers: Vec<SegmentId>,
}

const MAGIC_NUM: u64 = 3770719418566461763; // b'CIRCUIT4'

impl<C: GKRConfig> RecursiveCircuit<C> {
    pub fn load(filename: &str) -> Self {
        let mut ret = RecursiveCircuit::<C> {
            segments: Vec::new(),
            layers: Vec::new(),
        };
        let file_bytes = fs::read(filename).unwrap();
        let mut cursor = Cursor::new(file_bytes);

        let magic_num = u64::deserialize_from(&mut cursor);
        assert_eq!(magic_num, MAGIC_NUM);

        let field_mod = C::Field::deserialize_from_ecc_format(&mut cursor);
        log::trace!("field mod: {:?}", field_mod);
        let segment_num = u64::deserialize_from(&mut cursor);
        for _ in 0..segment_num {
            let seg = Segment::<C>::read(&mut cursor);
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

    fn is_parallel_repetition(&self, seg: &Segment<C>) -> bool {
        if seg.child_segs.len() == 0 {
            false
        } else {
            let child_seg_id = seg.child_segs[0].0;
            for child_seg in &seg.child_segs {
                if child_seg.0 != child_seg_id {
                    return false;
                }
            }

            let child_seg = &self.segments[seg.child_segs[0].0];
            if child_seg.child_segs.len() != 0 {
                println!("Loc 1: nb child child segs {}", child_seg.child_segs.len());
                // can actually support this recursive structure
                false
            } else {
                let child_seg_inpt_size = 1usize << child_seg.i_var_num;
                let child_seg_opt_size = 1usize << child_seg.o_var_num;
                let child_seg_allocs = seg
                    .child_segs
                    .iter()
                    .map(|(_seg, _alloc)| _alloc[0])
                    .collect::<Vec<Allocation>>();

                let is_parallel = child_seg_allocs.iter().enumerate().all(|(i, alloc)| {
                    alloc.i_offset == i * child_seg_inpt_size
                        && alloc.o_offset == i * child_seg_opt_size
                });

                is_parallel
            }
        }
    }

    fn flatten_into_layer_non_recursive(
        &self,
        layer_seg: &Segment<C>,
        i_offset: usize,
        o_offset: usize,
        ret_layer: &mut CircuitLayer<C>,
    ) {
        for gate in &layer_seg.gate_muls {
            let mut gate = gate.clone();
            gate.i_ids[0] += i_offset;
            gate.i_ids[1] += i_offset;
            gate.o_id += o_offset;
            ret_layer.mul.push(gate);
        }
        for gate in &layer_seg.gate_adds {
            let mut gate = gate.clone();
            gate.i_ids[0] += i_offset;
            gate.o_id += o_offset;
            ret_layer.add.push(gate);
        }

        for gate in &layer_seg.gate_consts {
            let mut gate = gate.clone();
            gate.o_id += o_offset;
            ret_layer.cst.push(gate);
        }

        for gate in &layer_seg.gate_uni {
            let mut gate = gate.clone();
            gate.i_ids[0] += i_offset;
            gate.o_id += o_offset;
            ret_layer.uni.push(gate);
        }
    }

    pub fn flatten(&self) -> Circuit<C> {
        let mut ret = Circuit::default();

        let mut nb_parallel_repetition_layers = 0;
        // layer-by-layer conversion
        for layer_id in &self.layers {
            let layer_seg: &Segment<C> = &self.segments[*layer_id];
            let mut ret_layer = CircuitLayer::<C>::new(layer_seg.i_var_num, layer_seg.o_var_num);

            if self.is_parallel_repetition(layer_seg) {
                nb_parallel_repetition_layers += 1;
                self.flatten_into_layer_non_recursive(layer_seg, 0, 0, &mut ret_layer);

                let child_seg = &self.segments[layer_seg.child_segs[0].0];
                ret_layer.nb_repetition = layer_seg.child_segs[0].1.len();
                ret_layer.sub_layer = Some(Box::new(CircuitLayer::<C>::new(
                    child_seg.i_var_num,
                    child_seg.o_var_num,
                )));
                self.flatten_into_layer_non_recursive(
                    child_seg,
                    0,
                    0,
                    &mut ret_layer.sub_layer.as_mut().unwrap(),
                );
            } else {
                let leaves: HashMap<usize, Vec<Allocation>> =
                    layer_seg.scan_leaf_segments(self, *layer_id);
                for (leaf_seg_id, leaf_allocs) in leaves {
                    let leaf_seg = &self.segments[leaf_seg_id];
                    for alloc in leaf_allocs {
                        self.flatten_into_layer_non_recursive(
                            leaf_seg,
                            alloc.i_offset,
                            alloc.o_offset,
                            &mut ret_layer,
                        )
                    }
                }
            }

            // debug print layer
            log::trace!(
                "total layers {}, parallel repetition layers: {} mul: {} add: {} const:{} uni:{} i_var_num: {} o_var_num: {}",
                ret.layers.len(),
                nb_parallel_repetition_layers,
                ret_layer.nb_mul_gates(),
                ret_layer.nb_add_gates(),
                ret_layer.nb_cst_gates(),
                ret_layer.nb_uni_gates(),
                ret_layer.input_var_num,
                ret_layer.output_var_num,
            );
            ret.layers.push(ret_layer);
        }

        ret.identify_rnd_coefs();
        ret
    }
}
