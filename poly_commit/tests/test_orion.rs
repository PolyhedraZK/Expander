use std::mem::transmute;

use arith::Field;
use ark_std::test_rng;
use gkr_engine::Transcript;
use mersenne31::{M31x16, M31, M31Ext3, M31Ext3x16};
use poly_commit::re_orion::*;
use gkr_hashers::{FiatShamirHasher, Keccak256hasher, SHA256hasher};
use transcript::BytesHashTranscript;

use p3_mersenne_31::Mersenne31;


#[test]
fn test_merkle_tree() {
    let n = 6;
    let mut mt = MerkleTree::<SHA256hasher>::new(n);
    let msg = vec![M31::ONE; n];
    mt.build(&msg);
    let proof = mt.prove(1 << mt.height, 1);
    let hasher = SHA256hasher::new();
    let mut leaf = vec![0u8; SHA256hasher::DIGEST_SIZE];
    let mut f = vec![0u8; M31::SIZE];
    msg[0].to_bytes(&mut f);
    hasher.hash(&mut leaf, &f);
    let verify = mt.verify(&mut leaf, &proof);
    assert!(verify);
}

#[test]
fn test_re_orion_e2e() {
    let mut rng = test_rng();
println!("e2e test start");
    type WitF = M31x16;
    type CodeF = M31x16;
    type EvalF = M31Ext3;
    type ResF = M31Ext3x16;

    // type WitF = M31;
    // type CodeF = M31;
    // type EvalF = M31;
    // type ResF = M31;

    // type WitF = M31x16;
    // type CodeF = M31x16;
    // type EvalF = M31;
    // type ResF = M31x16;

    // type WitF = M31;
    // type CodeF = M31;
    // type EvalF = M31Ext3;
    // type ResF = M31Ext3;

    let msg_bit = 11;
    let mut pcs = OrionInstance::<WitF, CodeF, EvalF, ResF, SHA256hasher>::new(1 << msg_bit);
    // let wit = vec![WitF::ONE; 1 << msg_bit];
    let wit: Vec<WitF> = (0..1 << msg_bit).map(|_| WitF::random_unsafe(&mut rng)).collect();
    let commit = pcs.commit(&wit);
println!("commit fin");

    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    // let poly = vec![EvalF::from(2); msg_bit];
    let poly: Vec<EvalF> = (0..msg_bit).map(|_| EvalF::random_unsafe(&mut rng)).collect();
    let opening = pcs.open(&commit, &poly, &mut transcript);
println!("open fin");
    
    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let verify = pcs.verify(&commit, &poly, &opening, &mut transcript);
    assert!(verify);
}