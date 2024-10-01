use std::fmt;
use std::fmt::Display;
use std::mem::transmute;

use babybear::BabyBearx16;
use poseidon::{PoseidonBabyBearParams, PoseidonBabyBearState};

use crate::Node;

/// A leaf is a blob of 64 bytes of data, stored in a BabyBearx16
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Leaf {
    pub(crate) data: BabyBearx16,
}

impl Display for Leaf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let t = unsafe { transmute::<BabyBearx16, [u8; 64]>(self.data) };
        write!(f, "leaf: 0x{:02x?}...{:02x?}", t[0], t[63])
    }
}

impl Leaf {
    pub fn new(data: BabyBearx16) -> Self {
        Self { data }
    }

    pub fn leaf_hash(&self, hash_param: &PoseidonBabyBearParams) -> Node {
        let mut state = PoseidonBabyBearState { state: self.data };
        hash_param.permute(&mut state);
        Node {
            data: unsafe {
                transmute::<BabyBearx16, [u8; 64]>(state.state)[..32]
                    .try_into()
                    .unwrap()
            },
        }
    }
}

impl From<BabyBearx16> for Leaf {
    fn from(data: BabyBearx16) -> Self {
        Self { data }
    }
}
