use std::marker::PhantomData;

use arith::Field;
use p3_challenger::{HashChallenger, SerializingChallenger32};
use p3_circle::CirclePcs;
use p3_commit::ExtensionMmcs;
use p3_field::{extension::{BinomialExtensionField, ComplexExtendable}, Algebra, ExtensionField, PrimeField32};

use mersenne31::{M31Ext3, M31Ext3x16, M31x16, M31};
use p3_fri::FriConfig;
use p3_keccak::Keccak256Hash;
use p3_merkle_tree::MerkleTreeMmcs;
use p3_mersenne_31::Mersenne31;
use p3_symmetric::{CompressionFunctionFromHasher, CryptographicHasher, SerializingHasher32};
use p3_uni_stark::StarkConfig;

use crate::utils::*;

pub struct P3Config {}

pub trait Plonky3Config<C: P3FieldConfig> {
    type Val: PrimeField32 = C::P3Field;
    type Challenge = C::P3Challenge;

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

impl<C: P3FieldConfig> Plonky3Config<C> for P3Config {
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

pub trait P3FieldConfig {
    type P3Field: PrimeField32 + ComplexExtendable;
    type P3Challenge: ExtensionField<Self::P3Field>;
}

impl P3FieldConfig for M31 {
    type P3Field = Mersenne31;
    type P3Challenge = BinomialExtensionField<Self::P3Field, 3>;
}

pub trait P3Multiply<RHS: Field> {
    fn p3mul<Expr: Algebra<Expr>>(lhs: &[Expr], rhs: &[Expr], res: &mut [Expr]);
}

impl P3Multiply<M31> for M31 {
    #[inline(always)]
    fn p3mul<Expr: Algebra<Expr>>(lhs: &[Expr], rhs: &[Expr], res: &mut [Expr]) {
        res[0] = lhs[0].clone() * rhs[0].clone()
    }
}

impl P3Multiply<M31x16> for M31 {
    #[inline(always)]
    fn p3mul<Expr: Algebra<Expr>>(lhs: &[Expr], rhs: &[Expr], res: &mut [Expr]) {
        for i in 0..16 {
            res[i] = lhs[0].clone() * rhs[i].clone()
        }
    }
}

impl P3Multiply<M31Ext3> for M31Ext3 {
    #[inline(always)]
    fn p3mul<Expr: Algebra<Expr>>(lhs: &[Expr], rhs: &[Expr], res: &mut [Expr]) {
        res[0] = lhs[0].clone() * rhs[0].clone() + mul5(lhs[1].clone() * rhs[2].clone() + lhs[2].clone() * rhs[1].clone());
        res[1] = lhs[0].clone() * rhs[1].clone() + lhs[1].clone() * rhs[0].clone() + mul5(lhs[2].clone() * rhs[2].clone());
        res[2] = lhs[0].clone() * rhs[2].clone() + lhs[1].clone() * rhs[1].clone() + lhs[2].clone() * rhs[0].clone();
    }
}

impl P3Multiply<M31Ext3x16> for M31Ext3 {
    #[inline(always)]
    fn p3mul<Expr: Algebra<Expr>>(lhs: &[Expr], rhs: &[Expr], res: &mut [Expr]) {
        for i in 0..16 {
            res[i] = lhs[0].clone() * rhs[i].clone() + mul5(lhs[1].clone() * rhs[32 + i].clone() + lhs[2].clone() * rhs[16 + i].clone());
            res[16 + i] = lhs[0].clone() * rhs[16 + i].clone() + lhs[1].clone() * rhs[i].clone() + mul5(lhs[2].clone() * rhs[32 + i].clone());
            res[32 + i] = lhs[0].clone() * rhs[32 + i].clone() + lhs[1].clone() * rhs[16 + i].clone() + lhs[2].clone() * rhs[i].clone();
        }
    }
}