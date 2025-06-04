use std::{borrow::Borrow, marker::PhantomData};

use arith::Field;
use p3_air::{Air, AirBuilder, AirBuilderWithPublicValues, BaseAir};
use p3_field::{extension::BinomiallyExtendable, PrimeField32, Algebra};
use p3_field::PrimeCharacteristicRing;
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use p3_uni_stark::{prove as p3prove, verify as p3verify, StarkConfig};

use p3_challenger::{SerializingChallenger32, HashChallenger};
use p3_circle::CirclePcs;
use p3_commit::ExtensionMmcs;
use p3_field::extension::{BinomialExtensionField, ComplexExtendable};
use p3_fri::FriConfig;
use p3_keccak::Keccak256Hash;
use p3_symmetric::{CompressionFunctionFromHasher, CryptographicHasher, SerializingHasher32};
use p3_merkle_tree::MerkleTreeMmcs;

const N: usize = 4;
const WIDTH: usize = 6;

struct TestAir {}

impl<F> BaseAir<F> for TestAir {
    fn width(&self) -> usize {
        WIDTH
    }
}

impl<AB: AirBuilderWithPublicValues> Air<AB> for TestAir {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let inputs = main.row_slice(0);
        let outputs = main.row_slice(1);

        let mut check = builder.when_first_row();

        let lhs = &inputs[0..3];
        let rhs = &inputs[3..6];
        let five = &outputs[3];
        check.assert_eq(outputs[0], lhs[0] * rhs[0] + *five * (lhs[1] * rhs[2] + lhs[2] * rhs[1]));
        let nn = lhs[2] * rhs[2];
        check.assert_eq(outputs[1], lhs[0] * rhs[1] + lhs[1] * rhs[0] + nn.double().double() + nn);
        check.assert_eq(outputs[2], lhs[0] * rhs[2] + lhs[1] * rhs[1] + lhs[2] * rhs[0]);

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
    let config = P3Config::<PF, Degree>::init();
    let proof = p3prove(&config, &TestAir{}, &mut P3Config::<PF, Degree>::get_challenger(), RowMajorMatrix::new(trace, 6), &vec![]);
    let proofu8 = serde_cbor::to_vec(&proof).unwrap();
    p3verify(&config, &TestAir{}, &mut P3Config::<PF, Degree>::get_challenger(), &serde_cbor::from_slice(&proofu8).unwrap(), &vec![]).is_ok()
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