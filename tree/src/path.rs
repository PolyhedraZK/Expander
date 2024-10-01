use core::fmt;
use std::fmt::Display;

use ark_std::{end_timer, start_timer};
use poseidon::PoseidonBabyBearParams;

use crate::{Leaf, Node};

#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub(crate) path_nodes: Vec<Node>,
    pub(crate) index: usize,
}

impl Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "leaf index: {}", self.index)?;

        let position_list = self.position_list();
        for ((i, node), is_right_node) in self.path_nodes.iter().enumerate().zip(position_list) {
            writeln!(
                f,
                "{}-th node, is right nide {}, sibling: {}",
                i, is_right_node, node
            )?;
        }

        Ok(())
    }
}

impl Path {
    /// The position of on_path node in `leaf_and_sibling_hash` and `non_leaf_and_sibling_hash_path`.
    /// `position[i]` is 0 (false) iff `i`th on-path node from top to bottom is on the left.
    ///
    /// This function simply converts `self.leaf_index` to boolean array in big endian form.
    #[inline]
    fn position_list(&'_ self) -> impl '_ + Iterator<Item = bool> {
        (0..self.path_nodes.len() + 1).map(move |i| ((self.index >> i) & 1) != 0)
    }

    /// verifies the path against a root
    #[inline]
    pub fn verify(&self, root: &Node, leaf: &Leaf, hasher: &PoseidonBabyBearParams) -> bool {
        let timer = start_timer!(|| "path verify");

        let position_list = self.position_list().collect::<Vec<_>>();
        let leaf_node = leaf.leaf_hash(hasher);
        let mut current_node = leaf_node;

        for (i, node) in self.path_nodes.iter().rev().enumerate() {
            if position_list[i] {
                current_node = Node::node_hash(node, &current_node)
            } else {
                current_node = Node::node_hash(&current_node, node)
            };
        }

        end_timer!(timer);
        if current_node != *root {
            println!("path does not match the root");
            false
        } else {
            true
        }
    }
}
