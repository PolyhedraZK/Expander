use gkr_engine::FieldEngine;
use serdes::{ExpSerde, SerdeResult};
use std::{cmp::max, collections::HashMap, fs, io::Cursor};

use crate::*;

// recursive format used in compiler
pub type SegmentId = usize;

#[derive(ExpSerde)]
pub struct Allocation {
    pub i_offset: usize,
    pub o_offset: usize,
}

#[derive(Default)]
pub struct Segment<C: FieldEngine> {
    pub i_var_num: usize,
    pub o_var_num: usize,
    pub child_segs: Vec<(SegmentId, Vec<Allocation>)>,
    pub gate_muls: Vec<GateMul<C>>,
    pub gate_adds: Vec<GateAdd<C>>,
    pub gate_consts: Vec<GateConst<C>>,
    pub gate_uni: Vec<GateUni<C>>,
}

impl<C: FieldEngine> Segment<C> {
    #[inline]
    pub fn contain_gates(&self) -> bool {
        !self.gate_muls.is_empty()
            || !self.gate_adds.is_empty()
            || !self.gate_consts.is_empty()
            || !self.gate_uni.is_empty()
    }

    #[inline]
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

#[derive(Default)]
pub struct RecursiveCircuit<C: FieldEngine> {
    pub num_public_inputs: usize,
    pub num_outputs: usize,
    pub expected_num_output_zeros: usize,

    pub segments: Vec<Segment<C>>,
    pub layers: Vec<SegmentId>,
}

impl<C: FieldEngine> RecursiveCircuit<C> {
    pub fn load(filename: &str) -> SerdeResult<Self> {
        let file_bytes = fs::read(filename)?;
        let cursor = Cursor::new(file_bytes);

        <Self as ExpSerde>::deserialize_from(cursor)
    }

    pub fn flatten(&self) -> Circuit<C> {
        let mut ret = Circuit::<C> {
            expected_num_output_zeros: self.expected_num_output_zeros,
            ..Default::default()
        };
        // layer-by-layer conversion
        for layer_id in &self.layers {
            let layer_seg = &self.segments[*layer_id];
            let leaves = layer_seg.scan_leaf_segments(self, *layer_id);
            let mut ret_layer = CircuitLayer {
                input_var_num: max(layer_seg.i_var_num, 1), // var_num >= 1
                output_var_num: max(layer_seg.o_var_num, 1), // var_num >= 1
                ..Default::default()
            };
            for (leaf_seg_id, leaf_allocs) in leaves {
                let leaf_seg = &self.segments[leaf_seg_id];
                for alloc in leaf_allocs {
                    for gate in &leaf_seg.gate_muls {
                        let mut gate = *gate;
                        gate.i_ids[0] += alloc.i_offset;
                        gate.i_ids[1] += alloc.i_offset;
                        gate.o_id += alloc.o_offset;
                        ret_layer.mul.push(gate);
                    }
                    for gate in &leaf_seg.gate_adds {
                        let mut gate = *gate;
                        gate.i_ids[0] += alloc.i_offset;
                        gate.o_id += alloc.o_offset;
                        ret_layer.add.push(gate);
                    }
                    for gate in &leaf_seg.gate_consts {
                        let mut gate = *gate;
                        gate.o_id += alloc.o_offset;
                        ret_layer.const_.push(gate);
                    }
                    for gate in &leaf_seg.gate_uni {
                        let mut gate = *gate;
                        gate.i_ids[0] += alloc.i_offset;
                        gate.o_id += alloc.o_offset;
                        ret_layer.uni.push(gate);
                    }
                }
            }
            // Prune input_var_num based on actual gate usage.
            // Scan all gates to find the maximum input index referenced,
            // then set input_var_num = ceil_log2(max_index + 1).
            // Relay layers (which reference all inputs) are unaffected.
            let max_input_idx = ret_layer.mul.iter()
                .flat_map(|g| g.i_ids.iter().copied())
                .chain(ret_layer.add.iter().flat_map(|g| g.i_ids.iter().copied()))
                .chain(ret_layer.uni.iter().flat_map(|g| g.i_ids.iter().copied()))
                .max();
            if let Some(max_idx) = max_input_idx {
                let needed = if max_idx == 0 {
                    1
                } else {
                    (max_idx + 1).next_power_of_two().trailing_zeros() as usize
                };
                // Only shrink, never expand
                if needed < ret_layer.input_var_num {
                    ret_layer.input_var_num = max(needed, 1);
                }
            }

            // debug print layer
            log::trace!(
                "layer {} mul: {} add: {} const:{} uni:{} i_var_num: {} o_var_num: {}",
                ret.layers.len(),
                ret_layer.mul.len(),
                ret_layer.add.len(),
                ret_layer.const_.len(),
                ret_layer.uni.len(),
                ret_layer.input_var_num,
                ret_layer.output_var_num,
            );
            ret.layers.push(ret_layer);
        }

        // Layer projection: shrink relay→compute transitions.
        // If compute layer uses subset of inputs, shrink relay output + re-index.
        // Reduces compute layer's input_var_num → less sumcheck data.
        // Relay layer's input_var_num unchanged → PCS commits all columns.
        for i in 0..ret.layers.len().saturating_sub(1) {
            // Collect input indices referenced by layer i+1's gates
            let mut used = std::collections::BTreeSet::new();
            {
                let next = &ret.layers[i + 1];
                for g in &next.mul { used.insert(g.i_ids[0]); used.insert(g.i_ids[1]); }
                for g in &next.add { used.insert(g.i_ids[0]); }
                for g in &next.uni { used.insert(g.i_ids[0]); }
            }
            if used.is_empty() { continue; }
            let cur_out = 1usize << ret.layers[i].output_var_num;
            let needed = (*used.iter().next_back().unwrap() + 1).next_power_of_two();
            if needed >= cur_out { continue; }
            let new_vn = max(needed.trailing_zeros() as usize, 1);
            // Compact mapping: old slot → new slot
            let used_vec: Vec<usize> = used.iter().copied().collect();
            let mut remap = vec![0usize; cur_out];
            for (ni, &oi) in used_vec.iter().enumerate() { remap[oi] = ni; }
            // Remap prev layer output gates (drop unused outputs)
            {
                let prev = &mut ret.layers[i];
                prev.add.retain(|g| used.contains(&g.o_id));
                for g in &mut prev.add { g.o_id = remap[g.o_id]; }
                prev.mul.retain(|g| used.contains(&g.o_id));
                for g in &mut prev.mul { g.o_id = remap[g.o_id]; }
                prev.const_.retain(|g| used.contains(&g.o_id));
                for g in &mut prev.const_ { g.o_id = remap[g.o_id]; }
                prev.uni.retain(|g| used.contains(&g.o_id));
                for g in &mut prev.uni { g.o_id = remap[g.o_id]; }
                prev.output_var_num = new_vn;
            }
            // Remap next layer input references
            {
                let next = &mut ret.layers[i + 1];
                for g in &mut next.mul { g.i_ids[0] = remap[g.i_ids[0]]; g.i_ids[1] = remap[g.i_ids[1]]; }
                for g in &mut next.add { g.i_ids[0] = remap[g.i_ids[0]]; }
                for g in &mut next.uni { g.i_ids[0] = remap[g.i_ids[0]]; }
                next.input_var_num = new_vn;
            }
            log::trace!("projection: layer {} output_var_num: {} → {} (used={}/{})",
                i, (cur_out as f64).log2() as usize, new_vn, used.len(), cur_out);
        }

        ret
    }
}
