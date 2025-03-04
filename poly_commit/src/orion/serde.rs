use std::io::{Read, Write};

use arith::Field;
use serdes::{ArithSerde, ExpSerde, SerdeResult};

use crate::orion::{
    linear_code::*,
    utils::{OrionProof, OrionSRS},
};

impl ExpSerde for OrionExpanderGraph {
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.l_vertices_size.serialize_into(&mut writer)?;
        self.r_vertices_size.serialize_into(&mut writer)?;
        self.neighborings.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let l_vertices_size = usize::deserialize_from(&mut reader)?;
        let r_vertices_size = usize::deserialize_from(&mut reader)?;
        let neighborings: Vec<DirectedNeighboring> =
            <Vec<DirectedNeighboring> as ArithSerde>::deserialize_from(&mut reader)?;
        Ok(Self {
            l_vertices_size,
            r_vertices_size,
            neighborings,
        })
    }
}

impl ExpSerde for OrionExpanderGraphPositioned {
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.input_starts.serialize_into(&mut writer)?;
        self.output_starts.serialize_into(&mut writer)?;
        self.output_ends.serialize_into(&mut writer)?;
        self.graph.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
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

impl ExpSerde for OrionCode {
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.hamming_weight.serialize_into(&mut writer)?;
        self.msg_len.serialize_into(&mut writer)?;
        self.codeword_len.serialize_into(&mut writer)?;
        self.g0s.serialize_into(&mut writer)?;
        self.g1s.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let hamming_weight = f64::deserialize_from(&mut reader)?;
        let msg_len = usize::deserialize_from(&mut reader)?;
        let codeword_len = usize::deserialize_from(&mut reader)?;
        let g0s: Vec<OrionExpanderGraphPositioned> =
            <Vec<OrionExpanderGraphPositioned> as ExpSerde>::deserialize_from(&mut reader)?;
        let g1s: Vec<OrionExpanderGraphPositioned> =
            <Vec<OrionExpanderGraphPositioned> as ExpSerde>::deserialize_from(&mut reader)?;
        Ok(Self {
            hamming_weight,
            msg_len,
            codeword_len,
            g0s,
            g1s,
        })
    }
}

impl ExpSerde for OrionSRS {
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.num_vars.serialize_into(&mut writer)?;
        self.code_instance.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let num_variables = usize::deserialize_from(&mut reader)?;
        let code_instance = OrionCode::deserialize_from(&mut reader)?;
        Ok(Self {
            num_vars: num_variables,
            code_instance,
        })
    }
}

impl<F: Field> ExpSerde for OrionProof<F> {
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.eval_row.serialize_into(&mut writer)?;
        self.proximity_rows.serialize_into(&mut writer)?;
        self.query_openings.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let eval_row: Vec<F> = <Vec<F> as ArithSerde>::deserialize_from(&mut reader)?;
        let proximity_rows: Vec<Vec<F>> =
            <Vec<Vec<F>> as ArithSerde>::deserialize_from(&mut reader)?;
        let query_openings: Vec<tree::RangePath> =
            <Vec<tree::RangePath> as ArithSerde>::deserialize_from(&mut reader)?;
        Ok(OrionProof {
            eval_row,
            proximity_rows,
            query_openings,
        })
    }
}
