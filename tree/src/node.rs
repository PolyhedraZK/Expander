use std::fmt;
use std::fmt::Display;

use sha2::{Digest, Sha512};

/// A node is a blob of 32 bytes of data
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Node {
    pub(crate) data: [u8; 32],
}

impl Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "node: 0x{:02x?}...{:02x?}", self.data[0], self.data[31])
    }
}

impl Node {
    pub fn new(data: [u8; 32]) -> Self {
        Self { data }
    }

    #[inline]
    pub fn node_hash(left: &Node, right: &Node) -> Node {
        // use sha2-512 to hash the two nodes
        let mut hasher = Sha512::new();
        hasher.update(left.data);
        hasher.update(right.data);
        let result = hasher.finalize();
        Node {
            data: result[..32].try_into().unwrap(),
        }
    }
}
