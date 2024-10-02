use std::fmt;
use std::fmt::Display;
use std::mem::transmute;

// use babybear::BabyBearx16;
// use poseidon::{PoseidonBabyBearParams, PoseidonBabyBearState};
use p3_baby_bear::PackedBabyBearAVX512 as BabyBearx16;
use sha2::{Digest, Sha512};

use crate::Node;

/// Represents a leaf in the Merkle tree, containing 64 bytes of data stored in a BabyBearx16.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Leaf {
    pub data: BabyBearx16,
}

impl Display for Leaf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Display the first and last byte of the leaf data for brevity
        let t = unsafe { transmute::<BabyBearx16, [u8; 64]>(self.data) };
        write!(f, "leaf: 0x{:02x?}...{:02x?}", t[0], t[63])
    }
}

impl Leaf {
    /// Creates a new Leaf with the given data.
    pub fn new(data: BabyBearx16) -> Self {
        Self { data }
    }

    pub fn leaf_hash(&self) -> Node {
        let mut hasher = Sha512::new();
        hasher.update(unsafe { transmute::<BabyBearx16, [u8; 64]>(self.data) });
        let result: [u8; 32] = hasher.finalize()[..32].try_into().unwrap();

        Node {
            data: result,
        }
    }

    // /// Computes the hash of the leaf using Poseidon hash function.
    // ///
    // /// # Arguments
    // ///
    // /// * `hash_param` - The Poseidon hash parameters
    // ///
    // /// # Returns
    // ///
    // /// A Node containing the hash of the leaf data.
    // pub fn leaf_hash(&self, hash_param: &PoseidonBabyBearParams) -> Node {
    //     // Use Poseidon hash for leaf nodes
    //     // Note: This could be replaced with SHA2 if performance requires
    //     let mut state = PoseidonBabyBearState { state: self.data };
    //     hash_param.permute(&mut state);
    //     Node {
    //         data: unsafe {
    //             transmute::<BabyBearx16, [u8; 64]>(state.state)[..32]
    //                 .try_into()
    //                 .unwrap()
    //         },
    //     }
    // }
}

impl From<BabyBearx16> for Leaf {
    /// Implements the From trait to allow creation of a Leaf from BabyBearx16 data.
    fn from(data: BabyBearx16) -> Self {
        Self { data }
    }
}
