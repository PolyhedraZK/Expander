use std::io::{Read, Write};

use super::circuit::{Circuit, CircuitLayer, StructureInfo};
use super::gates::{CoefType, Gate, GateAdd, GateConst, GateMul, GateUni};
use arith::{FieldSerde, FieldSerdeResult};
use gkr_field_config::GKRFieldConfig;

impl FieldSerde for CoefType {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        match self {
            CoefType::Constant => 0u8.serialize_into(&mut writer)?,
            CoefType::Random => 1u8.serialize_into(&mut writer)?,
            CoefType::PublicInput(idx) => {
                2u8.serialize_into(&mut writer)?;
                idx.serialize_into(&mut writer)?
            }
        };
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let tag = u8::deserialize_from(&mut reader)?;
        match tag {
            0 => Ok(CoefType::Constant),
            1 => Ok(CoefType::Random),
            2 => {
                let idx = usize::deserialize_from(&mut reader)?;
                Ok(CoefType::PublicInput(idx))
            }
            _ => panic!("Invalid tag for CoefType"),
        }
    }
}

impl<C: GKRFieldConfig, const INPUT_NUM: usize> FieldSerde for Gate<C, INPUT_NUM> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        self.i_ids
            .iter()
            .for_each(|i| i.serialize_into(&mut writer).unwrap());
        self.o_id.serialize_into(&mut writer)?;
        self.coef_type.serialize_into(&mut writer)?;
        self.coef.serialize_into(&mut writer)?;
        self.gate_type.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let mut i_ids = [0; INPUT_NUM];
        i_ids
            .iter_mut()
            .for_each(|i| *i = usize::deserialize_from(&mut reader).unwrap());
        let o_id = usize::deserialize_from(&mut reader)?;
        let coef_type = CoefType::deserialize_from(&mut reader)?;
        let coef = C::CircuitField::deserialize_from(&mut reader)?;
        let gate_type = usize::deserialize_from(&mut reader)?;
        Ok(Gate {
            i_ids,
            o_id,
            coef_type,
            coef,
            gate_type,
        })
    }
}

impl<C: GKRFieldConfig> FieldSerde for CircuitLayer<C> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        self.input_var_num.serialize_into(&mut writer)?;
        self.output_var_num.serialize_into(&mut writer)?;
        self.mul.serialize_into(&mut writer)?;
        self.add.serialize_into(&mut writer)?;
        self.const_.serialize_into(&mut writer)?;
        self.uni.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let input_var_num = usize::deserialize_from(&mut reader)?;
        let output_var_num = usize::deserialize_from(&mut reader)?;
        let mul = Vec::<GateMul<C>>::deserialize_from(&mut reader)?;
        let add = Vec::<GateAdd<C>>::deserialize_from(&mut reader)?;
        let const_ = Vec::<GateConst<C>>::deserialize_from(&mut reader)?;
        let uni = Vec::<GateUni<C>>::deserialize_from(&mut reader)?;
        Ok(CircuitLayer {
            input_var_num,
            output_var_num,

            input_vals: vec![],
            output_vals: vec![],

            mul,
            add,
            const_,
            uni,

            structure_info: StructureInfo::default(),
        })
    }
}

impl<C: GKRFieldConfig> FieldSerde for Circuit<C> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        self.layers.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let layers = Vec::<CircuitLayer<C>>::deserialize_from(&mut reader)?;
        Ok(Circuit {
            layers,

            ..Default::default()
        })
    }
}
