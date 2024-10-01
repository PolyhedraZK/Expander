use std::mem::size_of;
use std::mem::transmute;

use sha2::Digest as Sha2Digest;
use sha2::Sha256;

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Digest(pub [u8; OCTOPOS_OUTPUT_BYTES]);

impl Digest {
    pub fn as_u8s(&self) -> &[u8; OCTOPOS_OUTPUT_BYTES] {
        &self.0
    }
}

// NOTE: this const for poseidon hash state width is invariant of hash leaves' fields,
// namely, this is only with respect to the Goldilocks base field size, rather than
// arbitrary extension fields or curve element.
pub const OCTOPOS_HASH_STATE_GOLDILOCKS: usize = 12;
pub const OCTOPOS_HASH_RATE: usize = 11;

pub type PoseidonHasher = Poseidon<Goldilocks, OCTOPOS_HASH_STATE_GOLDILOCKS, OCTOPOS_HASH_RATE>;

pub const OCTOPOS_LEAF_GOLDILOCKS: usize = 8;

pub const OCTOPOS_LEAF_BYTES: usize = OCTOPOS_LEAF_GOLDILOCKS * size_of::<Goldilocks>();

pub const OCTOPOS_OUTPUT_GOLDILOCKS: usize = 4;

pub const OCTOPOS_OUTPUT_BYTES: usize = OCTOPOS_OUTPUT_GOLDILOCKS * size_of::<Goldilocks>();

pub const OCTOPOS_HASH_FULL_ROUNDS: usize = 8;

pub const OCTOPOS_HASH_PARTIAL_ROUNDS: usize = 22;

pub trait OctoposHasherTrait {
    fn new_instance() -> Self;
    fn name(&self) -> &'static str;
    fn hash_leaves(&self, leaves: &[u8; OCTOPOS_LEAF_BYTES]) -> Digest;
    fn hash_internals(&self, left: &Digest, right: &Digest) -> Digest;
}

impl OctoposHasherTrait for Sha256 {
    fn new_instance() -> Self {
        Sha256::new()
    }

    fn name(&self) -> &'static str {
        "Sha256"
    }

    fn hash_leaves(&self, leaves: &[u8; OCTOPOS_LEAF_BYTES]) -> Digest {
        let mut hasher = self.clone();

        hasher.update(leaves);
        let res = hasher.finalize();

        Digest(res.into())
    }

    fn hash_internals(&self, left: &Digest, right: &Digest) -> Digest {
        let mut hasher = self.clone();
        hasher.update([left.0.as_slice(), right.0.as_slice()].concat());
        let res = hasher.finalize();
        Digest(res.into())
    }
}

impl OctoposHasherTrait for Sha512_256 {
    fn new_instance() -> Self {
        Sha512_256::new()
    }

    fn name(&self) -> &'static str {
        "Sha512_256"
    }

    fn hash_leaves(&self, leaves: &[u8; OCTOPOS_LEAF_BYTES]) -> Digest {
        let mut hasher = self.clone();

        hasher.update(leaves);
        let res = hasher.finalize();

        Digest(res.into())
    }

    fn hash_internals(&self, left: &Digest, right: &Digest) -> Digest {
        let mut hasher = self.clone();
        hasher.update([left.0.as_slice(), right.0.as_slice()].concat());
        let res = hasher.finalize();
        Digest(res.into())
    }
}

impl OctoposHasherTrait for PoseidonHasher {
    fn new_instance() -> Self {
        PoseidonHasher::new(OCTOPOS_HASH_FULL_ROUNDS, OCTOPOS_HASH_PARTIAL_ROUNDS)
    }

    fn name(&self) -> &'static str {
        "Poseidon"
    }

    fn hash_leaves(&self, leaves: &[u8; OCTOPOS_LEAF_BYTES]) -> Digest {
        let mut hasher = self.clone();

        unsafe {
            let leaves_cast = transmute::<
                &[u8; OCTOPOS_LEAF_BYTES],
                &[Goldilocks; OCTOPOS_LEAF_GOLDILOCKS],
            >(leaves);

            hasher.update_without_permutation(leaves_cast);
            let res_goldilocks: &[Goldilocks; OCTOPOS_OUTPUT_GOLDILOCKS] = hasher
                .squeeze_vec_and_destroy()[..OCTOPOS_OUTPUT_GOLDILOCKS]
                .try_into()
                .unwrap();

            let res_cast = transmute::<
                &[Goldilocks; OCTOPOS_OUTPUT_GOLDILOCKS],
                &[u8; OCTOPOS_OUTPUT_BYTES],
            >(res_goldilocks);

            Digest(*res_cast)
        }
    }

    fn hash_internals(&self, left: &Digest, right: &Digest) -> Digest {
        let mut hasher = self.clone();

        unsafe {
            let left_cast = transmute::<
                &[u8; OCTOPOS_OUTPUT_BYTES],
                &[Goldilocks; OCTOPOS_OUTPUT_GOLDILOCKS],
            >(&left.0);

            let right_cast = transmute::<
                &[u8; OCTOPOS_OUTPUT_BYTES],
                &[Goldilocks; OCTOPOS_OUTPUT_GOLDILOCKS],
            >(&right.0);

            hasher.update_without_permutation(left_cast);
            hasher.update_without_permutation(right_cast);

            let res_goldilocks: &[Goldilocks; OCTOPOS_OUTPUT_GOLDILOCKS] = hasher
                .squeeze_vec_and_destroy()[..OCTOPOS_OUTPUT_GOLDILOCKS]
                .try_into()
                .unwrap();

            let res_cast = transmute::<
                &[Goldilocks; OCTOPOS_OUTPUT_GOLDILOCKS],
                &[u8; OCTOPOS_OUTPUT_BYTES],
            >(res_goldilocks);

            Digest(*res_cast)
        }
    }
}
