use std::marker::PhantomData;

use arith::Field;
use p3_field::{extension::BinomiallyExtendable, PrimeField32};
use p3_matrix::dense::RowMajorMatrix;
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

use super::air::CodeSwitchAir;

pub struct WitnessForPlonky3<'a, F: Field> {
    pub y_gamma: &'a [F],
    pub y1: &'a [F],
}

pub struct PublicValuesForPlonky3<'a, EvalF: Field, ResF: Field> {
    pub r1: &'a [EvalF],
    pub y: ResF,
    pub c_gamma: &'a [ResF],
}

// TODO: Encode bi-graph only constructed with base field?
pub fn prove<EvalF: Field, ResF: Field>(
    air: &CodeSwitchAir<EvalF, ResF>,
    witness: &WitnessForPlonky3<ResF>,
    public_values: &PublicValuesForPlonky3<EvalF, ResF>,
) -> Vec<u8> {
    match ResF::UnitField::NAME {
        "Mersenne 31" => prove_in_plonky3::<Mersenne31, 3, _, _>(air, witness, public_values),
        _ => unimplemented!()
    }
}

pub fn verify<EvalF: Field, ResF: Field>(
    air: &CodeSwitchAir<EvalF, ResF>,
    proof: &[u8],
    public_values: &PublicValuesForPlonky3<EvalF, ResF>,
) -> bool {
    match ResF::UnitField::NAME {
        "Mersenne 31" => verify_in_plonky3::<Mersenne31, 3, _, _>(air, proof, public_values),
        _ => unimplemented!()
    }
}

fn prove_in_plonky3<PF: PrimeField32 + ComplexExtendable + BinomiallyExtendable<Degree>, const Degree: usize, EvalF: Field, ResF: Field> (
    air: &CodeSwitchAir<EvalF, ResF>,
    witness: &WitnessForPlonky3<ResF>,
    public_values: &PublicValuesForPlonky3<EvalF, ResF>,
) -> Vec<u8> {
    let witness_size = ResF::get_degree() * ResF::get_pack_size();
    let width = air.msg_len * 2 * witness_size;
    let mut trace = PF::zero_vec(width * 4);
println!("prepare trace {}", width);
    unsafe { 
        let mut pos = 0;
        std::ptr::copy_nonoverlapping(witness.y_gamma.as_ptr() as *const PF, trace.as_mut_ptr(), witness_size * witness.y_gamma.len()); 
        pos += witness.y_gamma.len() * witness_size;
        std::ptr::copy_nonoverlapping(witness.y1.as_ptr() as *const PF, trace.as_mut_ptr().add(pos), witness_size * witness.y1.len()); 
        pos += witness.y1.len() * witness_size;
        std::ptr::copy_nonoverlapping(public_values.c_gamma.as_ptr() as *const PF, trace.as_mut_ptr().add(pos), public_values.c_gamma.len() * witness_size);
        pos += public_values.c_gamma.len() * witness_size;
    }
println!("trace fin");
    let challenge_size = EvalF::get_degree() * EvalF::get_pack_size();
    // TODO: borrow
    let mut pis = PF::zero_vec(public_values.r1.len() * challenge_size + (public_values.c_gamma.len() + 1) * witness_size);
// println!("{:?}", pis);
println!("prove pis len {} {} {} ", pis.len(), public_values.r1.len(), public_values.c_gamma.len());
    unsafe {
        std::ptr::copy_nonoverlapping(public_values.r1.as_ptr() as *const PF, pis.as_mut_ptr().add(air.code_len * witness_size), public_values.r1.len() * challenge_size);
// println!("r1 set {:?}", &pis[84..]);
        std::ptr::copy_nonoverlapping(vec![public_values.y].as_ptr() as *const PF, pis.as_mut_ptr().add(air.code_len * witness_size + public_values.r1.len() * challenge_size), witness_size);
    }
// println!("y {:?}", public_values.y);
// println!("{:?}", pis);
// println!("{:?}", &pis[84..]);

    let (trace_head, trace_tail) = trace.split_at_mut(width * 2);
    trace_tail[..width].copy_from_slice(&trace_head[width..]);
    trace_tail[width..width * 2].copy_from_slice(&trace_head[width..]);

    let config = P3Config::<PF, Degree>::init();

println!("plonky3 start");
    let proof = p3prove(&config, air, &mut P3Config::<PF, Degree>::get_challenger(), RowMajorMatrix::new(trace, width), &pis);
    serde_cbor::to_vec(&proof).unwrap()
}

fn verify_in_plonky3<PF: PrimeField32 + ComplexExtendable + BinomiallyExtendable<Degree>, const Degree: usize, EvalF: Field, ResF: Field> (
    air: &CodeSwitchAir<EvalF, ResF>,
    proof: &[u8],
    public_values: &PublicValuesForPlonky3<EvalF, ResF>,
) -> bool {
    let witness_size = ResF::get_degree() * ResF::get_pack_size();
    let challenge_size = EvalF::get_degree() * EvalF::get_pack_size();
    // TODO: borrow
    let mut pis = PF::zero_vec(public_values.r1.len() * challenge_size + (public_values.c_gamma.len() + 1) * witness_size);
println!("verify pis len {} {} {} ", pis.len(), public_values.r1.len(), public_values.c_gamma.len());
    unsafe {
        std::ptr::copy_nonoverlapping(public_values.r1.as_ptr() as *const PF, pis.as_mut_ptr().add(air.code_len * witness_size), public_values.r1.len() * challenge_size);
        std::ptr::copy_nonoverlapping(vec![public_values.y].as_ptr() as *const PF, pis.as_mut_ptr().add(air.code_len * witness_size + public_values.r1.len() * challenge_size), witness_size);
    }

    // let config = MyConfig::new(pcs, challenger);
    let config = P3Config::<PF, Degree>::init();

    let rst = p3verify(&config, air, &mut P3Config::<PF, Degree>::get_challenger(), &serde_cbor::from_slice(proof).unwrap(), &pis);
    if let Err(e) = rst {
        println!("{:?}", e);
        false
    }
    else {
        true
    }
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