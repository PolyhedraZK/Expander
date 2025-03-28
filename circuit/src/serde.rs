use arith::Field;
use gkr_field_config::GKRFieldConfig;
use serdes::{ExpSerde, SerdeResult};
use std::{
    io::{Read, Write},
    vec,
};

use super::{Allocation, CoefType, Gate, RecursiveCircuit, Segment, Witness};
use crate::{GateAdd, GateConst, GateMul, SegmentId};

pub struct CustomGateWrapper<C: GKRFieldConfig, const INPUT_NUM: usize> {
    pub custom_gate: Gate<C, INPUT_NUM>,
}

impl<C: GKRFieldConfig, const INPUT_NUM: usize> ExpSerde for CustomGateWrapper<C, INPUT_NUM> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut _writer: W) -> SerdeResult<()> {
        todo!()
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let gate_type = <usize as ExpSerde>::deserialize_from(&mut reader).unwrap();
        let i_ids: [usize; INPUT_NUM] = <Vec<usize> as ExpSerde>::deserialize_from(&mut reader)?
            .try_into()
            .unwrap();

        let o_id = <usize as ExpSerde>::deserialize_from(&mut reader)?;

        let coef_type_u8 = u8::deserialize_from(&mut reader)?;
        let (coef_type, coef) = match coef_type_u8 {
            1 => (
                CoefType::Constant,
                C::CircuitField::deserialize_from(&mut reader)?,
            ),
            2 => (CoefType::Random, C::CircuitField::ZERO),
            3 => {
                if INPUT_NUM > 0 {
                    panic!("Public Input can only be used with constant gates")
                };

                (
                    CoefType::PublicInput(<usize as ExpSerde>::deserialize_from(&mut reader)?),
                    C::CircuitField::ZERO,
                )
            }
            _ => unreachable!(),
        };

        Ok(Self {
            custom_gate: Gate::<C, INPUT_NUM> {
                i_ids,
                o_id,
                coef_type,
                coef,
                gate_type,
            },
        })
    }
}

impl ExpSerde for Allocation {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.i_offset.serialize_into(&mut writer)?;
        self.o_offset.serialize_into(&mut writer)?;

        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        Ok(Self {
            i_offset: <usize as ExpSerde>::deserialize_from(&mut reader)?,
            o_offset: <usize as ExpSerde>::deserialize_from(&mut reader)?,
        })
    }
}

impl<C: GKRFieldConfig> ExpSerde for Segment<C> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        <usize as ExpSerde>::serialize_into(&self.i_var_num, &mut writer)?;
        <usize as ExpSerde>::serialize_into(&self.o_var_num, &mut writer)?;

        self.child_segs.serialize_into(&mut writer)?;
        self.gate_muls.serialize_into(&mut writer)?;
        self.gate_adds.serialize_into(&mut writer)?;
        self.gate_consts.serialize_into(&mut writer)?;

        <usize as ExpSerde>::serialize_into(&self.gate_uni.len(), &mut writer)?;
        for uni in &self.gate_uni {
            CustomGateWrapper::<C, 1> { custom_gate: *uni }.serialize_into(&mut writer)?;
        }

        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let i_len = <usize as ExpSerde>::deserialize_from(&mut reader)?;
        let o_len = <usize as ExpSerde>::deserialize_from(&mut reader)?;
        assert!(i_len.is_power_of_two());
        assert!(o_len.is_power_of_two());

        let child_segs = Vec::<(SegmentId, Vec<Allocation>)>::deserialize_from(&mut reader)?;
        let gate_muls = Vec::<GateMul<C>>::deserialize_from(&mut reader)?;
        let gate_adds = Vec::<GateAdd<C>>::deserialize_from(&mut reader)?;
        let gate_consts = Vec::<GateConst<C>>::deserialize_from(&mut reader)?;

        let mut gate_uni = vec![];
        let len = <usize as ExpSerde>::deserialize_from(&mut reader)?;
        for _ in 0..len {
            let uni = CustomGateWrapper::<C, 1>::deserialize_from(&mut reader)?.custom_gate;
            gate_uni.push(uni);
        }
        Ok(Segment {
            i_var_num: i_len.trailing_zeros() as usize,
            o_var_num: o_len.trailing_zeros() as usize,
            child_segs,
            gate_muls,
            gate_adds,
            gate_consts,
            gate_uni,
        })
    }
}

const VERSION_NUM: usize = 3914834606642317635; // b'CIRCUIT6'

impl<C: GKRFieldConfig> ExpSerde for RecursiveCircuit<C> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        VERSION_NUM.serialize_into(&mut writer)?;
        C::CircuitField::MODULUS.serialize_into(&mut writer)?;

        self.num_public_inputs.serialize_into(&mut writer)?;
        self.num_outputs.serialize_into(&mut writer)?;
        self.expected_num_output_zeros.serialize_into(&mut writer)?;

        self.segments.serialize_into(&mut writer)?;
        self.layers.serialize_into(&mut writer)?;

        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let version_num = <usize as ExpSerde>::deserialize_from(&mut reader).unwrap();
        assert_eq!(version_num, VERSION_NUM);
        let expected_mod = <C::CircuitField as Field>::MODULUS;
        let mut read_mod = [0u8; 32];
        reader.read_exact(&mut read_mod).unwrap();
        assert_eq!(read_mod, expected_mod.to_le_bytes());

        Ok(RecursiveCircuit {
            num_public_inputs: <usize as ExpSerde>::deserialize_from(&mut reader).unwrap(),
            num_outputs: <usize as ExpSerde>::deserialize_from(&mut reader).unwrap(),
            expected_num_output_zeros: <usize as ExpSerde>::deserialize_from(&mut reader).unwrap(),

            segments: Vec::<Segment<C>>::deserialize_from(&mut reader)?,
            layers: <Vec<usize> as ExpSerde>::deserialize_from(&mut reader)?,
        })
    }
}

impl<C: GKRFieldConfig> ExpSerde for Witness<C> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut _writer: W) -> SerdeResult<()> {
        todo!()
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let num_witnesses = <usize as ExpSerde>::deserialize_from(&mut reader).unwrap();
        let num_private_inputs_per_witness =
            <usize as ExpSerde>::deserialize_from(&mut reader).unwrap();
        let num_public_inputs_per_witness =
            <usize as ExpSerde>::deserialize_from(&mut reader).unwrap();
        let _modulus = <[u64; 4]>::deserialize_from(&mut reader).unwrap();

        let mut values = vec![];
        for _ in 0..num_witnesses * (num_private_inputs_per_witness + num_public_inputs_per_witness)
        {
            values.push(C::CircuitField::deserialize_from(&mut reader).unwrap());
        }

        Ok(Self {
            num_witnesses,
            num_private_inputs_per_witness,
            num_public_inputs_per_witness,
            values,
        })
    }
}
