use std::io::{Read, Write};

use arith::{Field, FieldSerde, FieldSerdeResult};

use crate::orion::{
    linear_code::*,
    utils::{OrionProof, OrionSRS},
};

impl FieldSerde for OrionExpanderGraph {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        self.l_vertices_size.serialize_into(&mut writer)?;
        self.r_vertices_size.serialize_into(&mut writer)?;
        self.neighborings.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let l_vertices_size = usize::deserialize_from(&mut reader)?;
        let r_vertices_size = usize::deserialize_from(&mut reader)?;
        let neighborings: Vec<DirectedNeighboring> = Vec::deserialize_from(&mut reader)?;
        Ok(Self {
            l_vertices_size,
            r_vertices_size,
            neighborings,
        })
    }
}

impl FieldSerde for OrionExpanderGraphPositioned {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        self.input_starts.serialize_into(&mut writer)?;
        self.output_starts.serialize_into(&mut writer)?;
        self.output_ends.serialize_into(&mut writer)?;
        self.graph.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let input_starts = usize::deserialize_from(&mut reader)?;
        let output_starts = usize::deserialize_from(&mut reader)?;
        let output_ends = usize::deserialize_from(&mut reader)?;
        let graph = OrionExpanderGraph::deserialize_from(&mut reader)?;
        Ok(Self {
            input_starts,
            output_starts,
            output_ends,
            graph,
        })
    }
}

impl FieldSerde for OrionCode {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        self.hamming_weight.serialize_into(&mut writer)?;
        self.msg_len.serialize_into(&mut writer)?;
        self.codeword_len.serialize_into(&mut writer)?;
        self.g0s.serialize_into(&mut writer)?;
        self.g1s.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let hamming_weight = f64::deserialize_from(&mut reader)?;
        let msg_len = usize::deserialize_from(&mut reader)?;
        let codeword_len = usize::deserialize_from(&mut reader)?;
        let g0s: Vec<OrionExpanderGraphPositioned> = Vec::deserialize_from(&mut reader)?;
        let g1s: Vec<OrionExpanderGraphPositioned> = Vec::deserialize_from(&mut reader)?;
        Ok(Self {
            hamming_weight,
            msg_len,
            codeword_len,
            g0s,
            g1s,
        })
    }
}

impl FieldSerde for OrionSRS {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        self.num_vars.serialize_into(&mut writer)?;
        self.code_instance.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let num_variables = usize::deserialize_from(&mut reader)?;
        let code_instance = OrionCode::deserialize_from(&mut reader)?;
        Ok(Self {
            num_vars: num_variables,
            code_instance,
        })
    }
}

impl<F: Field> FieldSerde for OrionProof<F> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        self.eval_row.serialize_into(&mut writer)?;
        self.proximity_rows.serialize_into(&mut writer)?;
        self.query_openings.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let eval_row: Vec<F> = Vec::deserialize_from(&mut reader)?;
        let proximity_rows: Vec<Vec<F>> = Vec::deserialize_from(&mut reader)?;
        let query_openings: Vec<tree::RangePath> = Vec::deserialize_from(&mut reader)?;
        Ok(OrionProof {
            eval_row,
            proximity_rows,
            query_openings,
        })
    }
}
