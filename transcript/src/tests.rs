use arith::{BN254Fr, ExtensionField};
use field_hashers::{MiMC5FiatShamirHasher, PoseidonFiatShamirHasher};
use mersenne31::{M31Ext3, M31x16};
use sha2::{Digest, Sha256};

use crate::{BytesHashTranscript, FieldHashTranscript, Keccak256hasher, SHA256hasher, Transcript};

const EXAMPLE_IN: [u8; 32] = [
    40, 75, 185, 12, 169, 4, 108, 43, 211, 74, 219, 14, 2, 133, 97, 27, 200, 245, 110, 1, 253, 219,
    2, 24, 175, 47, 213, 9, 147, 218, 9, 24,
];
const EXAMPLE_OUT: [u8; 32] = [
    176, 91, 203, 102, 207, 182, 237, 150, 102, 95, 91, 217, 57, 237, 83, 244, 151, 151, 81, 14,
    152, 21, 4, 26, 66, 178, 223, 244, 32, 37, 40, 171,
];

#[test]
fn check_sha256_aligned() {
    let out = Sha256::digest(EXAMPLE_IN);
    println!("{:?}", out);
    assert_eq!(out, EXAMPLE_OUT.into());
}

fn test_transcript_expected_behavior_helper<F, T>()
where
    F: ExtensionField,
    T: Transcript<F>,
{
    {
        let mut transcript = T::new();

        let base_field_elems: Vec<F::BaseField> = vec![F::BaseField::from(1); F::DEGREE];
        let challenge_field_elem: F = F::from_limbs(&base_field_elems);

        transcript.append_field_element(&challenge_field_elem);
        let f = transcript.generate_challenge_field_element();

        transcript.append_field_element(&challenge_field_elem);
        let f2 = transcript.generate_challenge_field_element();

        transcript.append_field_element(&challenge_field_elem);
        let f3 = transcript.generate_challenge_field_element();

        assert_ne!(f, f2);
        assert_ne!(f, f3);
        assert_ne!(f2, f3);
    }
    {
        let mut transcript = T::new();

        transcript.append_u8_slice(b"input");
        let f = transcript.generate_challenge_field_element();

        transcript.append_u8_slice(b"input");
        let f2 = transcript.generate_challenge_field_element();

        transcript.append_u8_slice(b"input");
        let f3 = transcript.generate_challenge_field_element();

        assert_ne!(f, f2);
        assert_ne!(f, f3);
        assert_ne!(f2, f3);
    }
}

#[test]
fn test_transcript_expected_behavior() {
    test_transcript_expected_behavior_helper::<M31Ext3, BytesHashTranscript<_, Keccak256hasher>>();
    test_transcript_expected_behavior_helper::<M31Ext3, BytesHashTranscript<_, SHA256hasher>>();
    test_transcript_expected_behavior_helper::<BN254Fr, BytesHashTranscript<_, Keccak256hasher>>();
    test_transcript_expected_behavior_helper::<BN254Fr, BytesHashTranscript<_, SHA256hasher>>();

    test_transcript_expected_behavior_helper::<
        M31Ext3,
        FieldHashTranscript<_, PoseidonFiatShamirHasher<M31x16>>,
    >();
    test_transcript_expected_behavior_helper::<
        BN254Fr,
        FieldHashTranscript<_, MiMC5FiatShamirHasher<_>>,
    >();
}
