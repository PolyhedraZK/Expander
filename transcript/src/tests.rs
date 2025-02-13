use field_hashers::PoseidonFiatShamirHasher;
use mersenne31::{M31Ext3, M31x16, M31};
use sha2::{Digest, Sha256};

use crate::transcript::Transcript;
use crate::{BytesHashTranscript, FieldHashTranscript, Keccak256hasher};

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

#[test]
fn check_transcript_state() {
    {
        let mut transcript = BytesHashTranscript::<M31Ext3, Keccak256hasher>::new();

        transcript.append_field_element(&M31Ext3 {
            v: [M31::from(1), M31::from(2), M31::from(3)],
        });
        let f = transcript.generate_challenge_field_element();

        transcript.append_field_element(&M31Ext3 {
            v: [M31::from(1), M31::from(2), M31::from(3)],
        });
        let f2 = transcript.generate_challenge_field_element();

        transcript.append_field_element(&M31Ext3 {
            v: [M31::from(1), M31::from(2), M31::from(3)],
        });
        let f3 = transcript.generate_challenge_field_element();

        assert_ne!(f, f2);
        assert_ne!(f, f3);
        assert_ne!(f2, f3);
    }
    {
        let mut transcript = BytesHashTranscript::<M31Ext3, Keccak256hasher>::new();

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
    {
        let mut transcript =
            FieldHashTranscript::<M31Ext3, PoseidonFiatShamirHasher<M31x16>>::new();

        transcript.append_field_element(&M31Ext3 {
            v: [M31::from(1), M31::from(2), M31::from(3)],
        });
        let f = transcript.generate_challenge_field_element();

        transcript.append_field_element(&M31Ext3 {
            v: [M31::from(1), M31::from(2), M31::from(3)],
        });
        let f2 = transcript.generate_challenge_field_element();

        transcript.append_field_element(&M31Ext3 {
            v: [M31::from(1), M31::from(2), M31::from(3)],
        });
        let f3 = transcript.generate_challenge_field_element();

        assert_ne!(f, f2);
        assert_ne!(f, f3);
        assert_ne!(f2, f3);
    }
    {
        let mut transcript =
            FieldHashTranscript::<M31Ext3, PoseidonFiatShamirHasher<M31x16>>::new();

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
