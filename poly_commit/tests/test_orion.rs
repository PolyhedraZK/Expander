use arith::Field;
use gkr_engine::Transcript;
use mersenne31::M31;
use poly_commit::re_orion::*;
use gkr_hashers::{FiatShamirHasher, Keccak256hasher, SHA256hasher};
use transcript::BytesHashTranscript;

#[test]
fn test_merkle_tree() {
    let n = 6;
    let mut mt = MerkleTree::<SHA256hasher>::new(n);
    let msg = vec![M31::ONE; n];
    mt.build(&msg);
    let proof = mt.prove(1 << mt.height, 1);
    let hasher = SHA256hasher::new();
    let mut leaf = vec![0u8; SHA256hasher::DIGEST_SIZE];
    msg[0].to_bytes(&mut leaf[..M31::SIZE]);
    hasher.hash_inplace(&mut leaf);
    let verify = mt.verify(&mut leaf, &proof);
    assert!(verify);
}

#[test]
fn test_re_orion_e2e() {
    let msg_bit = 11;
    let mut pcs = OrionInstance::<M31, M31, M31, SHA256hasher>::new(1 << msg_bit);
    let wit = vec![M31::ONE; 1 << msg_bit];
    let commit = pcs.commit(&wit);

    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let poly = vec![M31::ONE; msg_bit];
    let opening = pcs.open(&commit, &poly, &mut transcript);
    
    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let verify = pcs.verify(&commit, &poly, &opening, &mut transcript);
    assert!(verify);
}