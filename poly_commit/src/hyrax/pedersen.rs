use halo2curves::{
    ff::{Field, PrimeField},
    group::Curve,
    msm, CurveAffine,
};
use serdes::ExpSerde;

use crate::StructuredReferenceString;

#[derive(Clone, Debug, Default)]
pub struct PedersenParams<C: CurveAffine + ExpSerde> {
    pub bases: Vec<C>,
}

impl<C: CurveAffine + ExpSerde> ExpSerde for PedersenParams<C> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> serdes::SerdeResult<()> {
        self.bases.serialize_into(&mut writer)
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> serdes::SerdeResult<Self> {
        let bases: Vec<C> = <Vec<C> as ExpSerde>::deserialize_from(&mut reader)?;
        Ok(Self { bases })
    }
}

impl<C: CurveAffine + ExpSerde> StructuredReferenceString for PedersenParams<C> {
    type PKey = Self;
    type VKey = Self;

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        (self.clone(), self)
    }
}

pub(crate) fn pedersen_setup<C: CurveAffine + ExpSerde>(
    length: usize,
    mut rng: impl rand::RngCore,
) -> PedersenParams<C>
where
    C::Scalar: PrimeField,
{
    let proj_bases: Vec<C::Curve> = (0..length)
        .map(|_| {
            let scalar = C::Scalar::random(&mut rng);
            C::generator() * scalar
        })
        .collect();

    let mut bases = vec![C::default(); length];
    C::Curve::batch_normalize(&proj_bases, &mut bases);

    PedersenParams { bases }
}

pub(crate) fn pedersen_commit<C: CurveAffine + ExpSerde>(
    params: &PedersenParams<C>,
    coeffs: &[C::Scalar],
) -> C
where
    C::Scalar: PrimeField,
{
    let mut what = C::default().to_curve();
    msm::multiexp_serial(coeffs, &params.bases, &mut what);

    what.to_affine()
}
