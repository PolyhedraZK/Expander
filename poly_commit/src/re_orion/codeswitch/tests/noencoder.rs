use std::marker::PhantomData;

use p3_air::{Air, AirBuilderWithPublicValues, BaseAir};
use p3_challenger::{HashChallenger, SerializingChallenger32};
use p3_circle::CirclePcs;
use p3_commit::ExtensionMmcs;
use p3_field::{Field, extension::{BinomialExtensionField, ComplexExtendable}, Algebra, ExtensionField, PrimeField32, PrimeCharacteristicRing};

use mersenne31::{M31Ext3, M31Ext3x16, M31x16, M31};
use p3_fri::FriConfig;
use p3_keccak::Keccak256Hash;
use p3_matrix::dense::RowMajorMatrix;
use p3_merkle_tree::MerkleTreeMmcs;
use p3_mersenne_31::Mersenne31;
use p3_symmetric::{CompressionFunctionFromHasher, CryptographicHasher, SerializingHasher32};
use p3_uni_stark::{prove, verify, StarkConfig};
use p3_matrix::Matrix;
use p3_air::AirBuilder;
use tracing_forest::{util::LevelFilter, ForestLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

use std::time::{Instant, Duration};

pub struct Timer {
    start: Instant,
    last: Duration,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            start: Instant::now(),
            last: Duration::new(0, 0),
        }
    }

    pub fn count(&mut self) -> Duration {
        let e = self.start.elapsed();
        let res = e - self.last;
        self.last = e;
        res
    }
}


struct CodeSwitchAir {
    // pub encoder: Encoder<<ResF::UnitField as P3FieldConfig>::P3Field>,
    pub encoder: Vec<Vec<Vec<(usize, u32)>>>,
    pub eval_degree: usize,
    pub res_pack_size: usize,
    
    pub msg_len: usize,
    pub code_len: usize,
    pub column_size: usize,

    pub idxs: Vec<usize>,
    // _marker: PhantomData<EvalF>,
}

const WIDTH: usize = 1 << 11;

impl<F> BaseAir<F> for CodeSwitchAir {
    fn width(&self) -> usize {
        // self.code_len * self.eval_degree * self.res_pack_size
        WIDTH
    }
}

impl<AB: AirBuilderWithPublicValues> Air<AB> for CodeSwitchAir {
    #[inline]
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let inputs = main.row_slice(0);
        let outputs = main.row_slice(1);

        let mut check = builder.when_first_row();
        for i in 0..outputs.len() {
            check.assert_zero(outputs[i]);
        }
    }
}

#[test]
fn test_air() {

let env_filter = EnvFilter::builder()
    .with_default_directive(LevelFilter::INFO.into())
    .from_env_lossy();
Registry::default()
    .with(env_filter)
    .with(ForestLayer::default())
    .init();

    let msg_len = 1 << (20 - 7) << 6;
    let mut encoder = vec![];
    for j in 0..10 {
        let mut v = vec![];
        for i in 0..msg_len >> j {
            v.push(vec![(0, 20); 30]);
        }
        encoder.push(v);
    }
    let air = CodeSwitchAir{
        encoder,
        eval_degree: 3,
        res_pack_size: 16,
        msg_len,
        code_len: msg_len * 2,
        column_size: 128,
        idxs: vec![0; 1500],
    };

    type Val = Mersenne31;
    type Challenge = BinomialExtensionField<Mersenne31, 3>;

    // Your choice of Hash Function
    type ByteHash = Keccak256Hash;
    type FieldHash = SerializingHasher32<ByteHash>;

    // Defines a compression function type using ByteHash, with 2 input blocks and 32-byte output.
    type MyCompress = CompressionFunctionFromHasher<ByteHash, 2, 32>;

    // Defines a Merkle tree commitment scheme for field elements with 32 levels.
    type ValMmcs = MerkleTreeMmcs<Val, u8, FieldHash, MyCompress, 32>;

    type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;

    // Defines the challenger type for generating random challenges.
    type MyHashChallenger = HashChallenger<u8, ByteHash, 32>;
    type Challenger = SerializingChallenger32<Val, MyHashChallenger>;

    type Pcs = CirclePcs<Val, ValMmcs, ChallengeMmcs>;

    type MyConfig = StarkConfig<Pcs, Challenge, Challenger>;

    let byte_hash = ByteHash {};
    let field_hash = FieldHash::new(ByteHash {});
    let compress = MyCompress::new(byte_hash);
    let val_mmcs = ValMmcs::new(field_hash, compress);
    let challenge_mmcs = ChallengeMmcs::new(val_mmcs.clone());
    let fri_config = FriConfig {
        log_blowup: 1,
        log_final_poly_len: 2,
        num_queries: 100,
        proof_of_work_bits: 16,
        mmcs: challenge_mmcs,
    };
    let pcs = Pcs {
        mmcs: val_mmcs,
        fri_config,
        _phantom: PhantomData,
    };
    let config = MyConfig::new(pcs);
    let mut challenger = Challenger::from_hasher(vec![], ByteHash {});

    let width = 4 * 16 * 2 * msg_len;
println!("{}", width * 4 % WIDTH);
let mut timer = Timer::new();
    let proof = prove(&config, &air, &mut challenger, RowMajorMatrix::new(Val::zero_vec(width * 4), WIDTH), &Val::zero_vec(width * 2));
println!("prove in {:?}", timer.count());
let mut timer = Timer::new();
    let mut challenger = Challenger::from_hasher(vec![], ByteHash {});
    assert!(verify(&config, &air, &mut challenger, &proof, &Val::zero_vec(width * 2)).is_ok());
println!("verify in {:?}", timer.count());
}