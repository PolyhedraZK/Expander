use arith::{BN254Fr, Field, FieldSerde};
use ark_std::test_rng;
use halo2curves::bn256::G1Affine;
use itertools::izip;
use transcript::Transcript;

use crate::hyrax::{pedersen::pedersen_vector_commit, PedersenParams};

pub struct PedersenIPAProof {
    pub com_r: G1Affine,
    pub sigma_3rd_masking: Vec<BN254Fr>,
}

#[allow(unused)]
pub(crate) fn pedersen_ipa_prove<T: Transcript<BN254Fr>>(
    pedersen_params: &PedersenParams,
    x_vec: &[BN254Fr],
    transcript: &mut T,
) -> PedersenIPAProof {
    let mut com_bytes: Vec<u8> = Vec::new();

    // NOTE(HS) we want some randomness in sigma protocol masking, but not from transcript.
    let mut p_rng = test_rng();
    let r_vec: Vec<_> = (0..x_vec.len())
        .map(|_| BN254Fr::random_unsafe(&mut p_rng))
        .collect();

    let com_r = pedersen_vector_commit(pedersen_params, &r_vec);
    com_r.serialize_into(&mut com_bytes).unwrap();
    transcript.append_u8_slice(&com_bytes);

    let mu = transcript.generate_challenge_field_element();
    let sigma_3rd_masking: Vec<BN254Fr> = izip!(&r_vec, x_vec).map(|(r, x)| r + mu * x).collect();

    PedersenIPAProof {
        com_r,
        sigma_3rd_masking,
    }
}

#[allow(unused)]
pub(crate) fn pedersen_ipa_verify<T: Transcript<BN254Fr>>(
    pedersen_params: &PedersenParams,
    com_x: G1Affine,
    proof: &PedersenIPAProof,
    y_vec: &[BN254Fr],
    eval: BN254Fr,
    transcript: &mut T,
) -> bool {
    let mut com_bytes: Vec<u8> = Vec::new();

    proof.com_r.serialize_into(&mut com_bytes).unwrap();
    transcript.append_u8_slice(&com_bytes);

    let mu = transcript.generate_challenge_field_element();
    let com_sigma_3rd = pedersen_vector_commit(pedersen_params, &proof.sigma_3rd_masking);
    let masked_comm: G1Affine = (com_x * mu + proof.com_r).into();

    com_sigma_3rd == masked_comm
}
