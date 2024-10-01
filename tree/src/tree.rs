use std::fmt::{self, Display};

use ark_std::{end_timer, start_timer};
use poseidon::PoseidonBabyBearParams;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator,
};

use crate::{Leaf, Node, Path};

#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    pub nodes: Vec<Node>,
    pub leaves: Vec<Leaf>, // todo: avoid cloning the data here
}

impl Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "nodes:")?;
        for (i, e) in self.nodes.iter().enumerate() {
            writeln!(f, "{}: {}", i, e)?;
        }
        writeln!(f, "leaves:")?;
        for (i, e) in self.leaves.iter().enumerate() {
            writeln!(f, "{}: {}", i, e)?;
        }
        Ok(())
    }
}

impl Tree {
    /// create an empty tree
    #[inline]
    pub fn init(hasher: &PoseidonBabyBearParams, tree_height: usize) -> Self {
        let leaves = vec![Leaf::default(); 1 << (tree_height - 1)];
        Self::new_with_leaves(hasher, leaves, tree_height)
    }

    /// build a tree with leaves
    #[inline]
    pub fn new_with_leaves(
        hasher: &PoseidonBabyBearParams,
        leaves: Vec<Leaf>,
        tree_height: usize,
    ) -> Self {
        let leaf_nodes = leaves
            .as_slice()
            .into_par_iter()
            .map(|leaf| leaf.leaf_hash(hasher))
            .collect::<Vec<Node>>();
        let nodes = Self::new_with_leaf_nodes(leaf_nodes, tree_height);
        Self {
            nodes: [nodes.0, nodes.1].concat(),
            leaves,
        }
    }

    /// build a tree with leaves
    /// assume the leaves are already hashed via leaf hash
    /// returns the leaf nodes and the tree nodes
    pub fn new_with_leaf_nodes(
        leaf_nodes: Vec<Node>,
        tree_height: usize,
    ) -> (Vec<Node>, Vec<Node>) {
        let timer = start_timer!(|| format!("generate new tree with {} leaves", leaf_nodes.len()));

        let len = leaf_nodes.len();
        assert_eq!(len, 1 << (tree_height - 1), "incorrect leaf size");

        let mut non_leaf_nodes = vec![Node::default(); (1 << (tree_height - 1)) - 1];

        // Compute the starting indices for each non-leaf level of the tree
        let mut index = 0;
        let mut level_indices = Vec::with_capacity(tree_height - 1);
        for _ in 0..(tree_height - 1) {
            level_indices.push(index);
            index = left_child_index(index);
        }

        // compute the hash values for the non-leaf bottom layer
        {
            let start_index = level_indices.pop().unwrap();
            let upper_bound = left_child_index(start_index);

            non_leaf_nodes
                .par_iter_mut()
                .enumerate()
                .take(upper_bound)
                .skip(start_index)
                .for_each(|(current_index, e)| {
                    // `left_child_index(current_index)` and `right_child_index(current_index) returns the position of
                    // leaf in the whole tree (represented as a list in level order). We need to shift it
                    // by `-upper_bound` to get the index in `leaf_nodes` list.
                    let left_leaf_index = left_child_index(current_index) - upper_bound;
                    let right_leaf_index = right_child_index(current_index) - upper_bound;
                    // compute hash
                    *e = Node::node_hash(
                        &leaf_nodes[left_leaf_index],
                        &leaf_nodes[right_leaf_index],
                    );
                });
        }

        // compute the hash values for nodes in every other layer in the tree
        level_indices.reverse();

        for &start_index in &level_indices {
            // The layer beginning `start_index` ends at `upper_bound` (exclusive).
            let upper_bound = left_child_index(start_index);
            let mut buf = non_leaf_nodes[start_index..upper_bound].to_vec();
            buf.par_iter_mut().enumerate().for_each(|(index, node)| {
                *node = Node::node_hash(
                    &non_leaf_nodes[left_child_index(index + start_index)],
                    &non_leaf_nodes[right_child_index(index + start_index)],
                );
            });
            non_leaf_nodes[start_index..upper_bound].clone_from_slice(buf.as_ref());
        }
        end_timer!(timer);

        (non_leaf_nodes, leaf_nodes.to_vec())
    }

    #[inline]
    pub fn root(&self) -> Node {
        self.nodes[0]
    }

    // generate a membership proof for the given index
    #[inline]
    pub fn gen_proof(&self, index: usize, tree_height: usize) -> Path {
        let timer = start_timer!(|| "generate membership proof");
        // Get Leaf hash, and leaf sibling hash,
        let leaf_index_in_tree = convert_index_to_last_level(index, tree_height);
        let sibling_index_in_tree = sibling_index(leaf_index_in_tree).unwrap();

        // path.len() = `tree height - 1`, the missing elements being the root
        let mut path_nodes = Vec::with_capacity(tree_height - 1);
        path_nodes.push(self.nodes[sibling_index_in_tree]);

        // Iterate from the bottom layer after the leaves, to the top, storing all nodes and their siblings.
        let mut current_node = parent_index(leaf_index_in_tree).unwrap();
        while current_node != 0 {
            let sibling_node = sibling_index(current_node).unwrap();

            path_nodes.push(self.nodes[sibling_node]);

            current_node = parent_index(current_node).unwrap();
        }

        // we want to make path from root to bottom
        path_nodes.reverse();
        end_timer!(timer);
        Path { index, path_nodes }
    }
}

/// Returns the index of the sibling, given an index.
#[inline]
fn sibling_index(index: usize) -> Option<usize> {
    if index == 0 {
        None
    } else if is_left_child(index) {
        Some(index + 1)
    } else {
        Some(index - 1)
    }
}

/// Returns the index of the parent, given an index.
#[inline]
fn parent_index(index: usize) -> Option<usize> {
    if index > 0 {
        Some((index - 1) >> 1)
    } else {
        None
    }
}

/// Returns the index of the left child, given an index.
#[inline]
fn left_child_index(index: usize) -> usize {
    2 * index + 1
}

/// Returns the index of the right child, given an index.
#[inline]
fn right_child_index(index: usize) -> usize {
    2 * index + 2
}

#[inline]
fn convert_index_to_last_level(index: usize, tree_height: usize) -> usize {
    index + (1 << (tree_height - 1)) - 1
}

/// Returns true iff the given index represents a left child.
#[inline]
fn is_left_child(index: usize) -> bool {
    index % 2 == 1
}
