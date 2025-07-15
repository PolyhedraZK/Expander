#![feature(associated_type_defaults)]

use std::mem::transmute;
use std::{borrow::Borrow, marker::PhantomData};

use arith::Field;
use codeswitch::P3Multiply;
use itertools::izip;
use mersenne31::{M31Ext3, M31};
use p3_air::{Air, AirBuilder, AirBuilderWithPublicValues, BaseAir};
use p3_examples::airs::ProofObjective;
use p3_field::{extension::BinomiallyExtendable, PrimeField32, Algebra};
use p3_field::PrimeCharacteristicRing;
use p3_keccak_air::KeccakAir;
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use p3_mersenne_31::Mersenne31;
use p3_uni_stark::{prove as p3prove, verify as p3verify, StarkConfig};

use p3_challenger::{SerializingChallenger32, HashChallenger};
use p3_circle::CirclePcs;
use p3_commit::ExtensionMmcs;
use p3_field::extension::{BinomialExtensionField, ComplexExtendable};
use p3_fri::FriConfig;
use p3_keccak::Keccak256Hash;
use p3_symmetric::{CompressionFunctionFromHasher, CryptographicHasher, SerializingHasher32};
use p3_merkle_tree::MerkleTreeMmcs;

use p3_examples::proofs::*;

const N: usize = 4;
const WIDTH: usize = 48;

struct TestAir {}

impl<F> BaseAir<F> for TestAir {
    fn width(&self) -> usize {
        WIDTH
    }
}

impl<AB: AirBuilderWithPublicValues> Air<AB> for TestAir {
    fn eval(&self, builder: &mut AB) {
        let public_values = builder.public_values();
        let mut out: Vec<AB::Expr> = public_values.iter().map(|&x| x.into()).collect();
        let rs = public_values[2];
        let main = builder.main();
        let inputs = main.row_slice(0);
        let outputs = main.row_slice(1);

        let mut check = builder.when_first_row();

        let eval_degree = 3;
        let witness_size = 3 * 16;

        let r1: Vec<AB::Expr> = outputs.iter().map(|&x| x.into()).collect();
        for (r1_chunk, y1_chunk) in izip!(r1[..3].chunks_exact(eval_degree), inputs.chunks_exact(witness_size)) {
            let mut res = r1_chunk.to_vec();
            for (y_unit, y1_unit) in izip!(out.chunks_exact_mut(eval_degree),y1_chunk.chunks_exact(eval_degree)) {
                let y1expr: Vec<AB::Expr> = y1_unit.iter().map(|&x| x.into()).collect();
                <M31Ext3 as P3Multiply<M31Ext3>>::p3mul(r1_chunk, &y1expr, &mut res);
                for (u, v) in izip!(y_unit.iter_mut(), res.iter()) {
                    *u -= v.clone();
                }
            }
        }
        for v in out.iter() {
            check.assert_zero(v.clone());
        }
            // for i in 0..3 {
            //     out[i] -= res[i].clone();
            // }
            // check.assert_eq(res[2].clone(), outputs[3].clone());
        // for (l_chunk, r_chunk) in izip!(inputs.chunks_exact(3), outputs.chunks_exact(3)) {
        //     let lhs: Vec<AB::Expr> = l_chunk.iter().map(|&x| x.into()).collect();
        //     let rhs: Vec<AB::Expr> = r_chunk.iter().map(|&x| x.into()).collect();
        //     let mut res = lhs.clone();
        //     M31Ext3::p3mul(&lhs, &rhs, &mut res);
        //     for i in 0..3 {
        //         out[i] -= res[i].clone();
        //     }
        //     break
        // }
        // for i in 2..3 {
        //     check.assert_zero(out[i].clone());
        // }
        // let five = &outputs[3];
        // // check.assert_eq(outputs[0], lhs[0] * rhs[0] + *five * (lhs[1] * rhs[2] + lhs[2] * rhs[1]));
        // // let nn = lhs[2] * rhs[2];
        // // check.assert_eq(outputs[1], lhs[0] * rhs[1] + lhs[1] * rhs[0] + nn.double().double() + nn);
        // check.assert_eq(outputs[2], lhs[0] * rhs[2] + lhs[1] * rhs[1] + lhs[2] * rhs[0]);

        // let mut y: AB::Expr = outputs[0].into();
        // for i in 0..N {
        //     y -= inputs[i] * inputs[i + N];
        // }
        // check.assert_zero(y);

        // for i in 0..N {
            // check.assert_eq(outputs[i], inputs[i] * inputs[i + N]);
// println!("{:?} ? {:?}", outputs[i].into(), inputs[i] * inputs[i + N]);
        // }

        // let mut check = builder.when_transition();
        // for i in 0..N {
        //     check.assert_eq(outputs[i], inputs[i]);
        // }
    }
}

pub fn prove_test<F: Field, PF: PrimeField32 + ComplexExtendable + BinomiallyExtendable<Degree>, const Degree: usize>(state: &[F]) -> bool {
    let mut trace = PF::zero_vec(state.len() * F::get_degree());
    unsafe { std::ptr::copy_nonoverlapping(state.as_ptr() as *const PF, trace.as_mut_ptr(), state.len() * F::get_degree()); }
    println!("{:?}", trace);
    true
}


trait Plonky3Config<F: PrimeField32, const D: usize> {
    type Val: PrimeField32 = F;
    type Challenge = BinomialExtensionField<Self::Val, D>;

    // Your choice of Hash Function
    type ByteHash: CryptographicHasher<u8, [u8; 32]> = Keccak256Hash;
    type FieldHash = SerializingHasher32<Self::ByteHash>;

    // Defines a compression function type using ByteHash, with 2 input blocks and 32-byte output.
    type MyCompress = CompressionFunctionFromHasher<Self::ByteHash, 2, 32>;

    // Defines a Merkle tree commitment scheme for field elements with 32 levels.
    type ValMmcs = MerkleTreeMmcs<Self::Val, u8, Self::FieldHash, Self::MyCompress, 32>;

    type ChallengeMmcs = ExtensionMmcs<Self::Val, Self::Challenge, Self::ValMmcs>;

    // Defines the challenger type for generating random challenges.
    type MyHashChallenger = HashChallenger<u8, Self::ByteHash, 32>;
    type Challenger = SerializingChallenger32<Self::Val, Self::MyHashChallenger>;

    type Pcs = CirclePcs<Self::Val, Self::ValMmcs, Self::ChallengeMmcs>;

    type MyConfig = StarkConfig<Self::Pcs, Self::Challenge, Self::Challenger>;

    fn init() -> Self::MyConfig;

    fn get_challenger() -> Self::Challenger;
}

struct P3Config<F: PrimeField32, const D: usize> {
    _marker: PhantomData<F>,
}

impl<F: PrimeField32, const D: usize> Plonky3Config<F, D> for P3Config<F, D> {
    #[inline(always)]
    fn init() -> Self::MyConfig {
        let byte_hash = Self::ByteHash {};
        let field_hash = Self::FieldHash::new(Self::ByteHash {});
        let compress = Self::MyCompress::new(byte_hash);
        let val_mmcs = Self::ValMmcs::new(field_hash, compress);
        let challenge_mmcs = Self::ChallengeMmcs::new(val_mmcs.clone());
        let fri_config = FriConfig {
            log_blowup: 1,
            log_final_poly_len: 2,
            num_queries: 100,
            proof_of_work_bits: 16,
            mmcs: challenge_mmcs,
        };
        let pcs = Self::Pcs {
            mmcs: val_mmcs,
            fri_config,
            _phantom: PhantomData,
        };
        // let challenger = Self::Challenger::new(Self::MyHashChallenger::new(vec![20180226u8; 32], Self::ByteHash{}));
        Self::MyConfig::new(pcs) // , challenger)
    }

    #[inline(always)]
    fn get_challenger() -> Self::Challenger {
        Self::Challenger::from_hasher(vec![], Self::ByteHash {})
    }
}

const NUM_FIBONACCI_COLS: usize = 2;
struct FibonacciAir {}

impl<F> BaseAir<F> for FibonacciAir {
    fn width(&self) -> usize {
        NUM_FIBONACCI_COLS
    }
}

impl<AB: AirBuilderWithPublicValues> Air<AB> for FibonacciAir {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();

        let pis = builder.public_values();

        let a = pis[0];
        let b = pis[1];
        let x = pis[2];

        let (local, next) = (
            main.row_slice(0),
            main.row_slice(1),
        );
        let local: &FibonacciRow<AB::Var> = (*local).borrow();
        let next: &FibonacciRow<AB::Var> = (*next).borrow();

        let mut when_first_row = builder.when_first_row();

        when_first_row.assert_eq(local.left, a);
        when_first_row.assert_eq(local.right, b);

        let mut when_transition = builder.when_transition();

        // a' <- b
        when_transition.assert_eq(local.right, next.left);

        // b' <- a + b
        when_transition.assert_eq(local.left + local.right, next.right);

        builder.when_last_row().assert_eq(local.right, x);
    }
}

pub fn prove_fib<F: Field, PF: PrimeField32 + ComplexExtendable + BinomiallyExtendable<Degree>, const Degree: usize>(state: &[F]) -> bool {
    /*
    let mut trace = RowMajorMatrix::new(PF::zero_vec(n * NUM_FIBONACCI_COLS), NUM_FIBONACCI_COLS);

    let (prefix, rows, suffix) = unsafe { trace.values.align_to_mut::<FibonacciRow<PF>>() };
    assert!(prefix.is_empty(), "Alignment should match");
    assert!(suffix.is_empty(), "Alignment should match");
    assert_eq!(rows.len(), n);

    rows[0] = FibonacciRow::new(PF::from_u32(a), PF::from_u32(b));

    for i in 1..n {
        rows[i].left = rows[i - 1].right;
        rows[i].right = rows[i - 1].left + rows[i - 1].right;
    } */

    let mut trace = PF::zero_vec(state.len());
    unsafe { std::ptr::copy_nonoverlapping(state.as_ptr() as *const PF, trace.as_mut_ptr(), state.len()); }
    let pis = vec![PF::ZERO, PF::ONE, PF::from_u32(21)];

    let config = P3Config::<PF, Degree>::init();
    let proof = p3prove(&config, &FibonacciAir{}, &mut P3Config::<PF, Degree>::get_challenger(), RowMajorMatrix::new(trace, NUM_FIBONACCI_COLS), &pis);
    p3verify(&config, &FibonacciAir{}, &mut P3Config::<PF, Degree>::get_challenger(), &proof, &pis).is_ok()
}

pub struct FibonacciRow<F> {
    pub left: F,
    pub right: F,
}

impl<F> FibonacciRow<F> {
    const fn new(left: F, right: F) -> Self {
        Self { left, right }
    }
}

impl<F> Borrow<FibonacciRow<F>> for [F] {
    fn borrow(&self) -> &FibonacciRow<F> {
        debug_assert_eq!(self.len(), NUM_FIBONACCI_COLS);
        let (prefix, shorts, suffix) = unsafe { self.align_to::<FibonacciRow<F>>() };
        debug_assert!(prefix.is_empty(), "Alignment should match");
        debug_assert!(suffix.is_empty(), "Alignment should match");
        debug_assert_eq!(shorts.len(), 1);
        &shorts[0]
    }
}

fn M31_from_vec(v: &[u32]) -> Vec<M31> {
    v.iter().map(|&x| M31::from(x)).collect()
}

// #[test]
fn test_multipier() {
    type TestField = Mersenne31;
    // let mut trace: Vec<M31> = [6, 3, 3, 8, 1, 5, 3, 7, 86, 15, 9, 56, 0, 0, 0, 0].iter().map(|&x| M31::from(x)).collect();
    let mut src = vec![1946252779, 39577958, 1157045030, 1869798449, 394603112, 56351660, 1094589052, 201533832, 972028740, 1883458276, 930416087, 773163055, 248795693, 1700447674, 47541112, 1258771633, 167346230, 760197990, 1163035780, 1069811190, 787016140, 2076452507, 1360283175, 664488066, 1895340633, 895741837, 1611630910, 632616314, 2001187800, 537130676, 781720858, 1605698409, 2134535476, 304768596, 676237896, 788241719, 702033570, 467239655, 882237774, 1560356918, 16239679, 1764572688, 98324301, 782685455, 1571319833, 1869784859, 160373417, 290379067];
    src.append(&mut vec![180902268, 146238478, 1799848064, 21562304, 547288797, 978984562, 1994681511, 1534662138, 1291408449, 96008516, 1916619159, 361661283, 661371992, 1000635125, 992602595, 934039460, 175923080, 1616030435, 878034992, 1763244420, 1300336980, 1668446665, 779439432, 1452800241, 1662887087, 555532054, 797038301, 462364805, 68221179, 1228982801, 2119822415, 2009529867, 1312550473, 1243827660, 1073015865, 1665683113, 1029426678, 719366829, 1276512264, 644882130, 1217427452, 673659680, 330929080, 1252281184, 1342860553, 2017925457, 272960470, 1236393029]);
    src.append(&mut vec![0; WIDTH * 2]);
    let mut trace = vec![TestField::ZERO; WIDTH * 4];
    unsafe { std::ptr::copy_nonoverlapping(src.as_ptr() as *const TestField, trace.as_mut_ptr(), src.len()); }

    // let pis = [1477386757, 360215737, 296761881].iter().map(|&x| TestField::from_u32(x)).collect();
    let pis = [1020243737, 1805229856, 21562304, 1526140531, 330236601, 19511576, 112968343, 269983071, 1991405778, 1807608985, 467111191, 320820517, 715558723, 1938082102, 1981837232, 885296928, 1498115866, 1501172329, 273120039, 693304095, 236223449, 1281953651, 1165628179, 595990458, 774411485, 1859595955, 43026855, 2121774197, 599424384, 1325199290, 1682325846, 1118339337, 493116410, 505968232, 1385120377, 999376038, 1778628078, 1179795703, 181979529, 2086659210, 1920994763, 2118610622, 1152869624, 2046218138, 32872305, 1493965913, 999661414, 2016188353].iter().map(|&x| TestField::from_u32(x)).collect();
    let config = P3Config::<TestField, 3>::init();
    let proof = p3prove(&config, &TestAir{}, &mut P3Config::<TestField, 3>::get_challenger(), RowMajorMatrix::new(trace, WIDTH), &pis);
    let proofu8 = serde_cbor::to_vec(&proof).unwrap();
    let rst = p3verify(&config, &TestAir{}, &mut P3Config::<TestField, 3>::get_challenger(), &serde_cbor::from_slice(&proofu8).unwrap(), &pis).is_ok();
    assert!(rst);

    // let a: M31Ext3 = unsafe { transmute([1, 1, 1])};
    // let b: M31Ext3 = unsafe { transmute([2, 2, 2])};
    // let c: M31Ext3 = a * b;
    // println!("{:?}", c);

    // let mut trace = vec![a, b, c, M31Ext3::from(5)];
    // trace.append(&mut trace[2..4].to_vec());
    // trace.append(&mut trace[2..4].to_vec());
    // let mut trace: Vec<M31> = [1, 1, 1, 2, 2, 2, 5, 7, 22, 14, 6, 56, 0, 0, 0, 0].iter().map(|&x| M31::from(x)).collect();
    // trace.append(&mut trace[8..16].to_vec());
    // trace.append(&mut trace[8..16].to_vec());
    // let rst = prove_test::<M31, Mersenne31, 3>(&trace);
    // let rst = prove_test::<_, Mersenne31, 3>(&trace);
    // let rst = true;
    // assert!(rst);
    // let mut trace: Vec<M31> = [0, 1, 1, 1, 1, 2, 2, 3, 3, 5, 5, 8, 8, 13, 13, 21].iter().map(|&x| M31::from(x)).collect();
    // assert!(prove_fib::<M31, Mersenne31, 3>(&trace));
}

/*
fn test_verify() {
    type EF = BinomialExtensionField<Mersenne31, 3>;
    let num_hashes = (1 << 20) / 24;
    let proof_goal = ProofObjective::Keccak(KeccakAir {});
    let result = prove_m31_keccak(proof_goal, num_hashes);
    report_result(result);
} */