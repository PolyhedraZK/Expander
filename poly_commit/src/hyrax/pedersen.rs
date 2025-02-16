use arith::FieldSerde;
use halo2curves::{
    ff::{Field, PrimeField},
    group::Curve,
    msm, CurveAffine,
};
use rand::thread_rng;

use crate::StructuredReferenceString;

#[derive(Clone, Debug, Default)]
pub struct PedersenParams<C: CurveAffine + FieldSerde> {
    pub bases: Vec<C>,
    pub h: C,
}

impl<C: CurveAffine + FieldSerde> FieldSerde for PedersenParams<C> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> arith::FieldSerdeResult<()> {
        self.bases.serialize_into(&mut writer)?;
        self.h.serialize_into(&mut writer)
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> arith::FieldSerdeResult<Self> {
        let bases: Vec<C> = Vec::deserialize_from(&mut reader)?;
        let h = C::deserialize_from(&mut reader)?;
        Ok(Self { bases, h })
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
    C::Scalar: PrimeField,
{
    let bases: Vec<C> = (0..=length)
        .map(|_| {
            let scalar = C::Scalar::random(&mut rng);
            (C::generator() * scalar).to_affine()
        })
        .collect();

    PedersenParams {
        h: bases[0],
        bases: bases[1..].to_vec(),
    }
}

pub(crate) fn pedersen_commit<C: CurveAffine + FieldSerde>(
    params: &PedersenParams<C>,
    coeffs: &[C::Scalar],
) -> (C, C::Scalar)
where
    C::Scalar: PrimeField,
{
    // NOTE(HS) we want some randomness in pedersen masking, but not from transcript.
    let r: C::Scalar = {
        let mut rng = thread_rng();
        C::Scalar::random(&mut rng)
    };

    let what = pedersen_commit_deterministic(params, coeffs, r);
    (what, r)
}

pub(crate) fn pedersen_commit_deterministic<C: CurveAffine + FieldSerde>(
    params: &PedersenParams<C>,
    coeffs: &[C::Scalar],
    r: C::Scalar,
) -> C
where
    C::Scalar: PrimeField,
{
    let mut what = C::default().to_curve();
    msm::multiexp_serial(coeffs, &params.bases[..coeffs.len()], &mut what);
    msm::multiexp_serial(&[r], &[params.h], &mut what);

    what.to_affine()
}
