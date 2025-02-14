use arith::{ExtensionField, Field, FieldSerde};
use ark_std::test_rng;
use halo2curves::{group::Curve, CurveAffine};
use itertools::izip;
use transcript::Transcript;

use crate::hyrax::{pedersen::pedersen_vector_commit, PedersenParams};

#[derive(Clone, Debug, Default)]
pub struct PedersenIPAProof<C: CurveAffine + FieldSerde>
where
    C::Scalar: FieldSerde,
{
    pub com_r: C,
    pub inner_prod_r_x: C::Scalar,
    pub sigma_3rd_masking: Vec<C::Scalar>,
}

impl<C: CurveAffine + FieldSerde> FieldSerde for PedersenIPAProof<C>
where
    C::Scalar: FieldSerde,
{
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> arith::FieldSerdeResult<()> {
        self.com_r.serialize_into(&mut writer)?;
        self.inner_prod_r_x.serialize_into(&mut writer)?;
        self.sigma_3rd_masking.serialize_into(&mut writer)
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> arith::FieldSerdeResult<Self> {
        let com_r: C = C::deserialize_from(&mut reader)?;
        let inner_prod_r_x: C::Scalar = C::Scalar::deserialize_from(&mut reader)?;
        let sigma_3rd_masking: Vec<C::Scalar> = Vec::deserialize_from(&mut reader)?;
        Ok(Self {
            com_r,
            inner_prod_r_x,
            sigma_3rd_masking,
        })
    }
}

pub(crate) fn pedersen_ipa_prove<C, T>(
    pedersen_params: &PedersenParams<C>,
    x_vec: &[C::Scalar],
    transcript: &mut T,
) -> PedersenIPAProof<C>
where
    C: CurveAffine + FieldSerde,
    T: Transcript<C::Scalar>,
    C::Scalar: ExtensionField,
    C::ScalarExt: ExtensionField,
{
    let mut com_bytes: Vec<u8> = Vec::new();

    // NOTE(HS) we want some randomness in sigma protocol masking, but not from transcript.
    let mut p_rng = test_rng();
    let r_vec: Vec<_> = (0..x_vec.len())
        .map(|_| C::Scalar::random_unsafe(&mut p_rng))
        .collect();

    let inner_prod_r_x = izip!(x_vec, &r_vec).map(|(x, r)| *x * *r).sum();
    let com_r = pedersen_vector_commit(pedersen_params, &r_vec);
    com_r.serialize_into(&mut com_bytes).unwrap();
    transcript.append_u8_slice(&com_bytes);

    let mu: C::Scalar = transcript.generate_challenge_field_element();
    let sigma_3rd_masking: Vec<_> = izip!(&r_vec, x_vec).map(|(r, x)| *r + mu * *x).collect();

    PedersenIPAProof {
        com_r,
        inner_prod_r_x,
        sigma_3rd_masking,
    }
}

pub(crate) fn pedersen_ipa_verify<C, T>(
    pedersen_params: &PedersenParams<C>,
    com_x: C,
    proof: &PedersenIPAProof<C>,
    y_vec: &[C::Scalar],
    eval: C::Scalar,
    transcript: &mut T,
) -> bool
where
    C: CurveAffine + FieldSerde,
    T: Transcript<C::Scalar>,
    C::Scalar: ExtensionField,
    C::ScalarExt: ExtensionField,
{
    let mut com_bytes: Vec<u8> = Vec::new();

    proof.com_r.serialize_into(&mut com_bytes).unwrap();
    transcript.append_u8_slice(&com_bytes);

    let mu = transcript.generate_challenge_field_element();
    let com_sigma_3rd = pedersen_vector_commit(pedersen_params, &proof.sigma_3rd_masking);
    if (com_x * mu + proof.com_r).to_affine() != com_sigma_3rd {
        return false;
    }

    let ip_expected = proof.inner_prod_r_x + mu * eval;
    let ip_actual: C::Scalar = izip!(&proof.sigma_3rd_masking, y_vec)
        .map(|(s, y)| *s * *y)
        .sum();

    ip_actual == ip_expected
}
