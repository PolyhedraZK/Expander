use arith::{ExtensionField, FieldSerde};
use halo2curves::{
    ff::{Field, PrimeField},
    group::Curve,
    CurveAffine,
};
use itertools::izip;
use rand::thread_rng;
use transcript::Transcript;

use crate::hyrax::{
    pedersen::{pedersen_commit, pedersen_commit_deterministic},
    PedersenParams,
};

#[derive(Clone, Debug, Default)]
pub struct PedersenIPAProof<C: CurveAffine + FieldSerde>
where
    C::Scalar: FieldSerde,
{
    pub com_r: C,
    pub com_zero: C,
    pub com_ip_ry: C,
    pub sigma_3rd_masked_r_mu_x: Vec<C::Scalar>,
    pub sigma_3rd_masked_r_mu_x_randomness: C::Scalar,
    pub sigma_3rd_masked_eval_randomness: C::Scalar,
}

impl<C: CurveAffine + FieldSerde> FieldSerde for PedersenIPAProof<C>
where
    C::Scalar: FieldSerde,
{
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> arith::FieldSerdeResult<()> {
        self.com_r.serialize_into(&mut writer)?;
        self.com_zero.serialize_into(&mut writer)?;
        self.com_ip_ry.serialize_into(&mut writer)?;
        self.sigma_3rd_masked_r_mu_x.serialize_into(&mut writer)
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> arith::FieldSerdeResult<Self> {
        let com_r: C = C::deserialize_from(&mut reader)?;
        let com_zero: C = C::deserialize_from(&mut reader)?;
        let com_ip_ry: C = C::deserialize_from(&mut reader)?;
        let sigma_3rd_masked_r_mu_x: Vec<C::Scalar> = Vec::deserialize_from(&mut reader)?;
        let sigma_3rd_masked_r_mu_x_randomness = C::Scalar::deserialize_from(&mut reader)?;
        let sigma_3rd_masked_eval_randomness = C::Scalar::deserialize_from(&mut reader)?;
        Ok(Self {
            com_r,
            com_zero,
            com_ip_ry,
            sigma_3rd_masked_r_mu_x,
            sigma_3rd_masked_r_mu_x_randomness,
            sigma_3rd_masked_eval_randomness,
        })
    }
}

pub(crate) fn pedersen_ipa_prove<C, T>(
    params: &PedersenParams<C>,
    x_vec: &[C::Scalar],
    y_vec: &[C::Scalar],
    com_randomness: C::Scalar,
    transcript: &mut T,
) -> PedersenIPAProof<C>
where
    C: CurveAffine + FieldSerde,
    T: Transcript<C::Scalar>,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
{
    let mut pedersen_commit_ro_absorb = |gs: &[C::Scalar]| -> (C, C::Scalar) {
        let (com, r_mask) = pedersen_commit(params, gs);

        let mut com_bytes: Vec<u8> = Vec::new();
        com.serialize_into(&mut com_bytes).unwrap();
        transcript.append_u8_slice(&com_bytes);
        (com, r_mask)
    };

    // NOTE(HS) we want some randomness in sigma protocol masking, but not from transcript.
    let r_vec: Vec<C::Scalar> = {
        let mut rng = thread_rng();
        (0..x_vec.len())
            .map(|_| C::Scalar::random(&mut rng))
            .collect()
    };
    let (com_r, rs_mask) = pedersen_commit_ro_absorb(&r_vec);

    let (com_zero, zero_mask) = pedersen_commit_ro_absorb(&[]);

    let ip_ry: C::Scalar = izip!(&r_vec, y_vec).map(|(r, y)| *r * *y).sum();
    let (com_ip_ry, ip_mask) = pedersen_commit_ro_absorb(&[ip_ry]);

    let mu: C::Scalar = transcript.generate_challenge_field_element();

    let sigma_3rd_masked_r_mu_x: Vec<_> = izip!(&r_vec, x_vec).map(|(r, x)| *r + mu * *x).collect();
    let sigma_3rd_masked_r_mu_x_randomness = com_randomness * mu + rs_mask;
    let sigma_3rd_masked_eval_randomness = zero_mask * mu + ip_mask;

    PedersenIPAProof {
        com_r,
        com_zero,
        com_ip_ry,
        sigma_3rd_masked_r_mu_x,
        sigma_3rd_masked_r_mu_x_randomness,
        sigma_3rd_masked_eval_randomness,
    }
}

pub(crate) fn pedersen_ipa_verify<C, T>(
    params: &PedersenParams<C>,
    com_x: C,
    proof: &PedersenIPAProof<C>,
    y_vec: &[C::Scalar],
    eval: C::Scalar,
    transcript: &mut T,
) -> bool
where
    C: CurveAffine + FieldSerde,
    T: Transcript<C::Scalar>,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
{
    let mut com_bytes: Vec<u8> = Vec::new();

    proof.com_r.serialize_into(&mut com_bytes).unwrap();
    transcript.append_u8_slice(&com_bytes);
    com_bytes.clear();

    proof.com_zero.serialize_into(&mut com_bytes).unwrap();
    transcript.append_u8_slice(&com_bytes);
    com_bytes.clear();

    proof.com_ip_ry.serialize_into(&mut com_bytes).unwrap();
    transcript.append_u8_slice(&com_bytes);
    com_bytes.clear();

    let mu = transcript.generate_challenge_field_element();

    let com_sigma_3rd_r_mu_x = pedersen_commit_deterministic(
        params,
        &proof.sigma_3rd_masked_r_mu_x,
        proof.sigma_3rd_masked_r_mu_x_randomness,
    );
    if (com_x * mu + proof.com_r).to_affine() != com_sigma_3rd_r_mu_x {
        return false;
    }

    let ip_r_mu_x_y: C::Scalar = izip!(&proof.sigma_3rd_masked_r_mu_x, y_vec)
        .map(|(s, y)| *s * *y)
        .sum();

    let com_masked_eval: C = pedersen_commit_deterministic(
        params,
        &[ip_r_mu_x_y],
        proof.sigma_3rd_masked_eval_randomness,
    );
    let com_mu_eval = pedersen_commit_deterministic(params, &[eval * mu], C::Scalar::ZERO);

    (proof.com_zero * mu + com_mu_eval + proof.com_ip_ry).to_affine() == com_masked_eval
}

#[cfg(test)]
mod test {
    use ark_std::test_rng;
    use halo2curves::{
        bn256::{Fr, G1Affine},
        ff::Field,
    };
    use itertools::izip;
    use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};

    use crate::hyrax::{
        inner_prod_argument::{pedersen_ipa_prove, pedersen_ipa_verify},
        pedersen::{pedersen_commit, pedersen_setup},
    };

    const IPA_VEC_LEN: usize = 1024;

    #[test]
    fn test_pedersen_sigma_ipa_e2e() {
        let mut rng = test_rng();
        let params = pedersen_setup::<G1Affine>(IPA_VEC_LEN, &mut rng);
        let mut p_transcript = BytesHashTranscript::<Fr, Keccak256hasher>::new();
        let mut v_transcript = p_transcript.clone();

        let x_vec: Vec<Fr> = (0..IPA_VEC_LEN).map(|_| Fr::random(&mut rng)).collect();
        let (x_com, x_com_randomness) = pedersen_commit(&params, &x_vec);

        let y_vec: Vec<Fr> = (0..IPA_VEC_LEN).map(|_| Fr::random(&mut rng)).collect();

        let expected_eval: Fr = izip!(&x_vec, &y_vec).map(|(x, y)| *x * *y).sum();

        let ipa_proof =
            pedersen_ipa_prove(&params, &x_vec, &y_vec, x_com_randomness, &mut p_transcript);

        let ipa_verification = pedersen_ipa_verify(
            &params,
            x_com,
            &ipa_proof,
            &y_vec,
            expected_eval,
            &mut v_transcript,
        );
        assert!(ipa_verification)
    }
}
