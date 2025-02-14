use arith::FieldSerde;
use halo2curves::{ff::PrimeField, group::Curve, msm, CurveAffine};

use crate::StructuredReferenceString;

#[derive(Clone, Debug, Default)]
pub struct PedersenParams<C: CurveAffine + FieldSerde>(pub Vec<C>);

impl<C: CurveAffine + FieldSerde> FieldSerde for PedersenParams<C> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, writer: W) -> arith::FieldSerdeResult<()> {
        self.0.serialize_into(writer)
    }

    fn deserialize_from<R: std::io::Read>(reader: R) -> arith::FieldSerdeResult<Self> {
        let buffer: Vec<C> = Vec::deserialize_from(reader)?;
        Ok(Self(buffer))
    }
}

impl<C: CurveAffine + FieldSerde> StructuredReferenceString for PedersenParams<C> {
    type PKey = Self;
    type VKey = Self;

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        (self.clone(), self)
    }
}

pub(crate) fn pedersen_setup<C: CurveAffine + FieldSerde>(
    length: usize,
    mut rng: impl rand::RngCore,
) -> PedersenParams<C>
where
    C::Scalar: PrimeField<Repr = [u8; 32]>,
{
    let mut buffer = [0u8; 32];
    let bases: Vec<C> = (0..length)
        .map(|_| {
            rng.fill_bytes(&mut buffer);
            let scalar = C::Scalar::from_repr(buffer).unwrap();
            (C::generator() * scalar).to_affine()
        })
        .collect();

    PedersenParams(bases)
}

pub(crate) fn pedersen_vector_commit<C: CurveAffine + FieldSerde>(
    params: &PedersenParams<C>,
    coeffs: &[C::Scalar],
) -> C {
    let mut what = C::default().to_curve();

    msm::multiexp_serial(coeffs, &params.0, &mut what);

    what.to_affine()
}
