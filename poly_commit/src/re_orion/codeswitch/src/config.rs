use std::marker::PhantomData;

use arith::Field;
use p3_air::BaseAir;
use p3_challenger::{HashChallenger, SerializingChallenger32};
use p3_circle::CirclePcs;
use p3_commit::{ExtensionMmcs, Pcs};
use p3_dft::Radix2DitParallel;
use p3_field::{extension::{BinomialExtensionField, ComplexExtendable}, Algebra, ExtensionField, PrimeField32};

use mersenne31::{M31Ext3, M31Ext3x16, M31x16, M31};
use p3_fri::{FriConfig, TwoAdicFriPcs};
use p3_keccak::{Keccak256Hash, KeccakF};
use p3_matrix::dense::RowMajorMatrix;
use p3_merkle_tree::MerkleTreeMmcs;
use p3_mersenne_31::Mersenne31;
use p3_symmetric::{CompressionFunctionFromHasher, CryptographicHasher, PaddingFreeSponge, SerializingHasher32, SerializingHasher32To64};
use p3_uni_stark::{StarkConfig, StarkGenericConfig, prove, verify};

use crate::{utils::*, CodeSwitchAir};

pub trait P3Config {
    type Val: PrimeField32 + ComplexExtendable;
    type Challenge: ExtensionField<Self::Val>;
    // type Challenger;
    // type PCS: Pcs<Self::Challenge, Self::Challenger>;
    type Config: StarkGenericConfig<Challenge = Self::Challenge>;

    fn init_stark_config() -> (Self::Config, <Self::Config as StarkGenericConfig>::Challenger);

    fn p3prove<EvalF, ResF>(
        air: &CodeSwitchAir<EvalF, ResF>,
        width: usize,
        trace: Vec<Self::Val>,
        pis: &Vec<Self::Val>,
    ) -> Vec<u8> 
    where
        EvalF: Field<UnitField = Self> + P3Multiply<ResF> + P3Multiply<EvalF>,
        ResF: Field<UnitField = Self>;

    fn p3verify<EvalF, ResF>(
        air: &CodeSwitchAir<EvalF, ResF>,
        proof: &[u8],
        pis: &Vec<Self::Val>,
    ) -> bool
    where
        EvalF: Field<UnitField = Self> + P3Multiply<ResF> + P3Multiply<EvalF>,
        ResF: Field<UnitField = Self>;
}

/*
impl<C: P3FieldConfig> Plonky3Config<C> for P3Config {
    #[inline(always)]
    fn init() -> (Self::MyConfig, Self::Challenger) {
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
        // let dft = Self::Dft::default();
        // let pcs = Self::Pcs::new(dft, val_mmcs, fri_config);
        let pcs = Self::Pcs {
            mmcs: val_mmcs,
            fri_config,
            _phantom: PhantomData,
        };
        // let challenger = Self::Challenger::new(Self::MyHashChallenger::new(vec![20180226u8; 32], Self::ByteHash{}));
        (Self::MyConfig::new(pcs), Self::Challenger::from_hasher(vec![], Self::ByteHash {}))
    }
} */

type KeccakCompressionFunction =
    CompressionFunctionFromHasher<PaddingFreeSponge<KeccakF, 25, 17, 4>, 2, 4>;
type KeccakMerkleMmcs<F> = MerkleTreeMmcs<
    [F; p3_keccak::VECTOR_LEN],
    [u64; p3_keccak::VECTOR_LEN],
    SerializingHasher32To64<PaddingFreeSponge<KeccakF, 25, 17, 4>>,
    KeccakCompressionFunction,
    4,
>;
type KeccakCircleStarkConfig<F, EF> = StarkConfig<
    CirclePcs<F, KeccakMerkleMmcs<F>, ExtensionMmcs<F, EF, KeccakMerkleMmcs<F>>>,
    EF,
    SerializingChallenger32<F, HashChallenger<u8, Keccak256Hash, 32>>,
>;


impl P3Config for M31 {
    type Val = Mersenne31;
    type Challenge = BinomialExtensionField<Self::Val, 3>;
    // type Challenger = SerializingChallenger32<Self::Val, HashChallenger<u8, Keccak256Hash, 32>>;
    type Config = KeccakCircleStarkConfig<Self::Val, Self::Challenge>;

    #[inline(always)]
    fn init_stark_config() -> (Self::Config, <Self::Config as StarkGenericConfig>::Challenger) {
        let u64_hash = PaddingFreeSponge::<KeccakF, 25, 17, 4>::new(KeccakF {});
        let field_hash = SerializingHasher32To64::new(u64_hash);
        let compress = KeccakCompressionFunction::new(u64_hash);
        let val_mmcs = KeccakMerkleMmcs::new(field_hash, compress);
        let challenge_mmcs = ExtensionMmcs::<Self::Val, Self::Challenge, _>::new(val_mmcs.clone());
        let fri_config = FriConfig {
            log_blowup: 1,
            log_final_poly_len: 0,
            num_queries: 100,
            proof_of_work_bits: 16,
            mmcs: challenge_mmcs,
        };
        let pcs = CirclePcs::new(val_mmcs, fri_config);
        let challenger = SerializingChallenger32::from_hasher(vec![], Keccak256Hash {});
        let config = KeccakCircleStarkConfig::new(pcs);
        (config, challenger)
    }

    fn p3prove<EvalF, ResF>(
        air: &CodeSwitchAir<EvalF, ResF>,
        width: usize,
        trace: Vec<Self::Val>,
        pis: &Vec<Self::Val>,
    ) -> Vec<u8> 
    where
        EvalF: Field<UnitField = Self> + P3Multiply<ResF> + P3Multiply<EvalF>,
        ResF: Field<UnitField = Self>,
    {
        let u64_hash = PaddingFreeSponge::<KeccakF, 25, 17, 4>::new(KeccakF {});
        let field_hash = SerializingHasher32To64::new(u64_hash);
        let compress = KeccakCompressionFunction::new(u64_hash);
        let val_mmcs = KeccakMerkleMmcs::new(field_hash, compress);
        let challenge_mmcs = ExtensionMmcs::<Self::Val, Self::Challenge, _>::new(val_mmcs.clone());
        let fri_config = FriConfig {
            log_blowup: 1,
            log_final_poly_len: 0,
            num_queries: 100,
            proof_of_work_bits: 16,
            mmcs: challenge_mmcs,
        };
        let pcs = CirclePcs::new(val_mmcs, fri_config);
        let mut challenger = SerializingChallenger32::from_hasher(vec![], Keccak256Hash {});
        let config = KeccakCircleStarkConfig::new(pcs);

        let proof = prove(&config, air, &mut challenger, RowMajorMatrix::new(trace, width), pis);
        serde_cbor::to_vec(&proof).unwrap()
    }

    fn p3verify<EvalF, ResF>(
        air: &CodeSwitchAir<EvalF, ResF>,
        proofu8: &[u8],
        pis: &Vec<Self::Val>,
    ) -> bool
    where
        EvalF: Field<UnitField = Self> + P3Multiply<ResF> + P3Multiply<EvalF>,
        ResF: Field<UnitField = Self>,
    {
        let u64_hash = PaddingFreeSponge::<KeccakF, 25, 17, 4>::new(KeccakF {});
        let field_hash = SerializingHasher32To64::new(u64_hash);
        let compress = KeccakCompressionFunction::new(u64_hash);
        let val_mmcs = KeccakMerkleMmcs::new(field_hash, compress);
        let challenge_mmcs = ExtensionMmcs::<Self::Val, Self::Challenge, _>::new(val_mmcs.clone());
        let fri_config = FriConfig {
            log_blowup: 1,
            log_final_poly_len: 0,
            num_queries: 100,
            proof_of_work_bits: 16,
            mmcs: challenge_mmcs,
        };
        let pcs = CirclePcs::new(val_mmcs, fri_config);
        let mut challenger = SerializingChallenger32::from_hasher(vec![], Keccak256Hash {});
        let config = KeccakCircleStarkConfig::new(pcs);

        let proof = serde_cbor::from_slice(proofu8).unwrap();
let mut timer = Timer::new();
        let rst = verify(&config, air, &mut challenger, &proof, &pis);
println!("p3verify in {:?}", timer.count());

        if let Err(e) = rst {
            println!("{:?}", e);
            false
        }
        else {
            true
        }
    }
    
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