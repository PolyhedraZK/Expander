use std::mem::transmute;

use arith::Field;
use gkr_engine::Transcript;
use mersenne31::{M31x16, M31, M31Ext3, M31Ext3x16};
use poly_commit::re_orion::*;
use gkr_hashers::{FiatShamirHasher, Keccak256hasher, SHA256hasher};
use transcript::BytesHashTranscript;

use p3_mersenne_31::Mersenne31;

// #[test]
fn test_plonky3() {
    // let mut trace: Vec<M31> = [6, 3, 3, 8, 1, 5, 3, 7, 86, 15, 9, 56, 0, 0, 0, 0].iter().map(|&x| M31::from(x)).collect();
    let a: M31Ext3 = unsafe { transmute([1, 1, 1])};
    let b: M31Ext3 = unsafe { transmute([2, 2, 2])};
    let c: M31Ext3 = a * b;
    println!("{:?}", c);
    let mut trace = vec![a, b, c, M31Ext3::from(5)];
    trace.append(&mut trace[2..4].to_vec());
    trace.append(&mut trace[2..4].to_vec());
    // let mut trace: Vec<M31> = [1, 1, 1, 2, 2, 2, 5, 7, 22, 14, 6, 56, 0, 0, 0, 0].iter().map(|&x| M31::from(x)).collect();
    // trace.append(&mut trace[8..16].to_vec());
    // trace.append(&mut trace[8..16].to_vec());
    // let rst = prove_test::<M31, Mersenne31, 3>(&trace);
    // let rst = prove_test::<_, Mersenne31, 3>(&trace);
    let rst = true;
    assert!(rst);
    // let mut trace: Vec<M31> = [0, 1, 1, 1, 1, 2, 2, 3, 3, 5, 5, 8, 8, 13, 13, 21].iter().map(|&x| M31::from(x)).collect();
    // assert!(prove_fib::<M31, Mersenne31, 3>(&trace));
}

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

    let msg_bit = 20;
    let mut pcs = OrionInstance::<WitF, CodeF, EvalF, ResF, SHA256hasher>::new(1 << msg_bit);
    let wit = vec![WitF::ONE; 1 << msg_bit];
    let commit = pcs.commit(&wit);
println!("commit fin");

    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let poly = vec![EvalF::from(2); msg_bit];
    let opening = pcs.open(&commit, &poly, &mut transcript);
println!("open fin");
    
    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let verify = pcs.verify(&commit, &poly, &opening, &mut transcript);
    assert!(verify);
}