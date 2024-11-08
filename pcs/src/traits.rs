use arith::FieldSerde;
use rand::RngCore;
use std::fmt::Debug;

pub trait PCS {
    type Params: Clone + Debug;
    type Poly: Clone + Debug;
    type EvalPoint: Clone + Debug;
    type Eval: Copy + Clone + Debug;

    type SRS: Clone + Debug + FieldSerde;
    type PKey: Clone + Debug + From<Self::SRS> + FieldSerde;
    type VKey: Clone + Debug + From<Self::SRS> + FieldSerde;
    type Commitment: Clone + Debug + FieldSerde;
    type Opening: Clone + Debug + FieldSerde;

    fn gen_srs_for_testing(&mut self, rng: impl RngCore, params: &Self::Params) -> Self::SRS;

    fn commit(
        &mut self,
        params: &Self::Params,
        proving_key: &Self::PKey,
        poly: &Self::Poly,
    ) -> Self::Commitment;

    fn open(
        &mut self,
        params: &Self::Params,
        proving_key: &Self::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
    ) -> (Self::Eval, Self::Opening);

    fn verify(
        params: &Self::Params,
        verifying_key: &Self::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: Self::Eval,
        opening: &Self::Opening,
    ) -> bool;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EmptyType {}

impl FieldSerde for EmptyType {
    const SERIALIZED_SIZE: usize = 0;

    fn serialize_into<W: std::io::Write>(&self, _writer: W) -> arith::FieldSerdeResult<()> {
        Ok(())
    }

    fn deserialize_from<R: std::io::Read>(_reader: R) -> arith::FieldSerdeResult<Self> {
        Ok(Self {})
    }

    fn try_deserialize_from_ecc_format<R: std::io::Read>(
        _reader: R,
    ) -> arith::FieldSerdeResult<Self> {
        unimplemented!()
    }
}
