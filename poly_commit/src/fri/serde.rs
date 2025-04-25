use arith::{FFTField, Field};
use serdes::ExpSerde;
use tree::{Node, RangePath};

use crate::fri::{FRIOpening, FRIScratchPad};

impl<F: FFTField> ExpSerde for FRIScratchPad<F> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> serdes::SerdeResult<Self> {
        let merkle: tree::Tree = tree::Tree::deserialize_from(&mut reader)?;
        let codeword: Vec<F> = Vec::deserialize_from(&mut reader)?;
        let rate_log2: usize = usize::deserialize_from(&mut reader)?;

        Ok(Self {
            merkle,
            codeword,
            rate_log2,
        })
    }

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> serdes::SerdeResult<()> {
        self.merkle.serialize_into(&mut writer)?;
        self.codeword.serialize_into(&mut writer)?;
        self.rate_log2.serialize_into(&mut writer)?;

        Ok(())
    }
}

impl<F: Field> ExpSerde for FRIOpening<F> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> serdes::SerdeResult<Self> {
        let iopp_oracles: Vec<Node> = Vec::deserialize_from(&mut reader)?;
        let iopp_queries: Vec<Vec<(RangePath, RangePath)>> = Vec::deserialize_from(&mut reader)?;
        let sumcheck_responses: Vec<Vec<F>> = Vec::deserialize_from(&mut reader)?;

        Ok(Self {
            iopp_oracles,
            iopp_queries,
            sumcheck_responses,
        })
    }

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> serdes::SerdeResult<()> {
        self.iopp_oracles.serialize_into(&mut writer)?;
        self.iopp_queries.serialize_into(&mut writer)?;
        self.sumcheck_responses.serialize_into(&mut writer)?;

        Ok(())
    }
}
