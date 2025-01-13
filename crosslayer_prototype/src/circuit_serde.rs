use arith::{Field, FieldForECC, FieldSerde, FieldSerdeError};
use gkr_field_config::GKRFieldConfig;
use std::{io::Read, vec};
use thiserror::Error;

use super::{Allocation, CoefType, CrossLayerRecursiveCircuit, CrossLayerSegment, Witness};
use crate::{CrossLayerRelay, SegmentId, SimpleGate};

#[derive(Debug, Error)]
pub enum CircuitError {
    #[error("field serde error: {0:?}")]
    FieldSerdeError(#[from] FieldSerdeError),

    #[error("other error: {0:?}")]
    OtherError(#[from] std::io::Error),
}
pub trait FromEccSerde {
    fn deserialize_from<R: Read>(reader: R) -> Self;
}

impl<T: FromEccSerde> FromEccSerde for Vec<T> {
    fn deserialize_from<R: Read>(mut reader: R) -> Self {
        let vec_len = <usize as FieldSerde>::deserialize_from(&mut reader).unwrap();
        let mut ret = vec![];
        for _ in 0..vec_len {
            ret.push(T::deserialize_from(&mut reader));
        }
        ret
    }
}

impl<T1: FromEccSerde, T2: FromEccSerde> FromEccSerde for (T1, T2) {
    fn deserialize_from<R: Read>(mut reader: R) -> Self {
        (
            T1::deserialize_from(&mut reader),
            T2::deserialize_from(&mut reader),
        )
    }
}

impl FromEccSerde for usize {
    fn deserialize_from<R: Read>(reader: R) -> Self {
        <usize as FieldSerde>::deserialize_from(reader).unwrap()
    }
}

/// A gate whose inputs are from different layers.
#[derive(Debug, Clone)]
pub struct ECCCrossLayerGate<C: GKRFieldConfig, const INPUT_NUM: usize> {
    pub i_ids: [(usize, usize); INPUT_NUM], // (layer_offset, gate_offset)
    pub o_id: usize,
    pub coef_type: CoefType,
    pub coef: C::CircuitField,
}

impl<C, const INPUT_NUM: usize> ECCCrossLayerGate<C, INPUT_NUM>
where
    C: GKRFieldConfig,
{
    pub fn to_simple_gate_or_relay(
        &self,
    ) -> (Option<SimpleGate<C, INPUT_NUM>>, Option<CrossLayerRelay<C>>) {
        if INPUT_NUM == 1 && self.i_ids[0].0 != 0 {
            (
                None,
                Some(CrossLayerRelay {
                    o_id: self.o_id,
                    i_id: self.i_ids[0].1,
                    i_layer: self.i_ids[0].0,
                    coef: self.coef,
                }),
            )
        } else {
            let mut i_ids = [0; INPUT_NUM];
            for (i, i_id) in i_ids.iter_mut().enumerate() {
                let (_layer_offset, gate_offset) = self.i_ids[i];
                *i_id = gate_offset;
            }

            let gate = SimpleGate {
                i_ids,
                o_id: self.o_id,
                coef_type: self.coef_type.clone(),
                coef: self.coef,
            };
            (Some(gate), None)
        }
    }
}

impl<C: GKRFieldConfig, const INPUT_NUM: usize> FromEccSerde for ECCCrossLayerGate<C, INPUT_NUM> {
    fn deserialize_from<R: Read>(mut reader: R) -> Self {
        let mut i_ids = [(0usize, 0usize); INPUT_NUM];
        for (layer_offset, gate_offset) in &mut i_ids {
            *layer_offset = <usize as FieldSerde>::deserialize_from(&mut reader).unwrap();
            *gate_offset = <usize as FieldSerde>::deserialize_from(&mut reader).unwrap();
        }

        let o_id = <usize as FieldSerde>::deserialize_from(&mut reader).unwrap();

        let coef_type_u8 = u8::deserialize_from(&mut reader).unwrap();
        let (coef_type, coef) = match coef_type_u8 {
            1 => (
                CoefType::Constant,
                C::CircuitField::deserialize_from(&mut reader).unwrap(),
            ),
            2 => (CoefType::Random, C::CircuitField::ONE),
            3 => {
                if INPUT_NUM > 0 {
                    panic!("Public Input can only be used with constant gates")
                };

                (
                    CoefType::PublicInput(
                        <usize as FieldSerde>::deserialize_from(&mut reader).unwrap(),
                    ),
                    C::CircuitField::ZERO,
                )
            }
            _ => unreachable!(),
        };

        Self {
            i_ids,
            o_id,
            coef_type,
            coef,
        }
    }
}

impl FromEccSerde for Allocation {
    fn deserialize_from<R: Read>(mut reader: R) -> Self {
        Self {
            i_offset: <Vec<usize> as FieldSerde>::deserialize_from(&mut reader).unwrap(),
            o_offset: <usize as FieldSerde>::deserialize_from(&mut reader).unwrap(),
        }
    }
}

impl<C: GKRFieldConfig> FromEccSerde for CrossLayerSegment<C> {
    fn deserialize_from<R: Read>(mut reader: R) -> Self {
        let input_size = <Vec<usize> as FieldSerde>::deserialize_from(&mut reader).unwrap();
        let output_size = <usize as FieldSerde>::deserialize_from(&mut reader).unwrap();
        assert!(input_size.iter().all(|&x| x.is_power_of_two()));
        assert!(output_size.is_power_of_two());

        let child_segs = Vec::<(SegmentId, Vec<Allocation>)>::deserialize_from(&mut reader);
        let ecc_gate_muls = Vec::<ECCCrossLayerGate<C, 2>>::deserialize_from(&mut reader);
        let ecc_gate_adds = Vec::<ECCCrossLayerGate<C, 1>>::deserialize_from(&mut reader);
        let ecc_gate_csts = Vec::<ECCCrossLayerGate<C, 0>>::deserialize_from(&mut reader);

        let gate_muls = ecc_gate_muls
            .into_iter()
            .map(|gate| gate.to_simple_gate_or_relay().0.unwrap())
            .collect();

        let mut gate_adds = vec![];
        let mut gate_relay = vec![];
        for g in ecc_gate_adds.iter() {
            let (gate, relay) = g.to_simple_gate_or_relay();
            if let Some(gate) = gate {
                gate_adds.push(gate);
            }
            if let Some(relay) = relay {
                gate_relay.push(relay);
            }
        }

        let gate_csts = ecc_gate_csts
            .into_iter()
            .map(|gate| gate.to_simple_gate_or_relay().0.unwrap())
            .collect();

        let custom_gates_size = <usize as FieldSerde>::deserialize_from(&mut reader).unwrap();
        assert_eq!(custom_gates_size, 0);

        CrossLayerSegment {
            input_size,
            output_size,
            child_segs,
            gate_muls,
            gate_adds,
            gate_csts,
            gate_relay,
        }
    }
}

const VERSION_NUM: usize = 3914834606642317635; // b'CIRCUIT6'

impl<C: GKRFieldConfig> FromEccSerde for CrossLayerRecursiveCircuit<C> {
    fn deserialize_from<R: Read>(mut reader: R) -> Self {
        let version_num = <usize as FieldSerde>::deserialize_from(&mut reader).unwrap();
        assert_eq!(version_num, VERSION_NUM);
        let expected_mod = <C::CircuitField as FieldForECC>::MODULUS;
        let mut field_mod = [0u8; 32];
        reader.read_exact(&mut field_mod).unwrap();
        let read_mod = ethnum::U256::from_le_bytes(field_mod);
        assert_eq!(expected_mod, read_mod);

        CrossLayerRecursiveCircuit {
            num_public_inputs: <usize as FieldSerde>::deserialize_from(&mut reader).unwrap(),
            num_outputs: <usize as FieldSerde>::deserialize_from(&mut reader).unwrap(),
            expected_num_output_zeros: <usize as FieldSerde>::deserialize_from(&mut reader)
                .unwrap(),

            segments: Vec::<CrossLayerSegment<C>>::deserialize_from(&mut reader),
            layers: <Vec<usize> as FromEccSerde>::deserialize_from(&mut reader),
        }
    }
}

impl<C: GKRFieldConfig> FromEccSerde for Witness<C> {
    fn deserialize_from<R: Read>(mut reader: R) -> Self {
        let num_witnesses = <usize as FieldSerde>::deserialize_from(&mut reader).unwrap();
        let num_private_inputs_per_witness =
            <usize as FieldSerde>::deserialize_from(&mut reader).unwrap();
        let num_public_inputs_per_witness =
            <usize as FieldSerde>::deserialize_from(&mut reader).unwrap();
        let _modulus = <[u64; 4]>::deserialize_from(&mut reader).unwrap();

        let mut values = vec![];
        for _ in 0..num_witnesses * (num_private_inputs_per_witness + num_public_inputs_per_witness)
        {
            values.push(C::CircuitField::deserialize_from(&mut reader).unwrap());
        }

        Self {
            num_witnesses,
            num_private_inputs_per_witness,
            num_public_inputs_per_witness,
            values,
        }
    }
}
