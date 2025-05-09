use std::fmt;
use std::fmt::{Debug, Display};

use crate::{
    common_ancestor, convert_index_to_last_level, is_left_child, parent_index, Leaf, Node, Tree,
};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct RangePath {
    pub leaves: Vec<Leaf>,
    pub(crate) path_nodes: Vec<Node>,
    pub left: usize,
}

impl Display for RangePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "leaf index range: [{}, {}]",
            self.left,
            self.left + self.leaves.len() - 1
        )?;

        let position_list = self.position_list();
        for ((i, node), is_right_node) in self.path_nodes.iter().enumerate().zip(position_list) {
            writeln!(
                f,
                "{i}-th node, is right node {is_right_node}, sibling: {node}"
            )?;
        }

        Ok(())
    }
}

impl RangePath {
    #[inline]
    fn position_list(&'_ self) -> impl '_ + Iterator<Item = bool> {
        let common_ancestor = common_ancestor(self.left, self.left + self.leaves.len() - 1);

        (0..self.path_nodes.len() + 1).map(move |i| ((common_ancestor >> i) & 1) != 0)
    }

    #[inline]
    pub fn root(&self) -> Node {
        let (tree_height, mut current_node) = if self.leaves.len() > 1 {
            let sub_tree = Tree::new_with_leaves(self.leaves.clone());
            let tree_height = sub_tree.height() + self.path_nodes.len();

            let root = sub_tree.root();

            drop(sub_tree);

            (tree_height, root)
        } else {
            let root = self.leaves[0].leaf_hash();
            let tree_height = self.path_nodes.len() + 1;

            (tree_height, root)
        };

        let left_index_in_tree = convert_index_to_last_level(self.left, tree_height);
        let right_index_in_tree =
            convert_index_to_last_level(self.left + self.leaves.len() - 1, tree_height);

        let mut current_node_index = common_ancestor(left_index_in_tree, right_index_in_tree);

        self.path_nodes.iter().rev().for_each(|node| {
            if is_left_child(current_node_index) {
                current_node = Node::node_hash(&current_node, node)
            } else {
                current_node = Node::node_hash(node, &current_node)
            }

            current_node_index = parent_index(current_node_index).unwrap();
        });

        current_node
    }

    #[inline]
    pub fn verify(&self, root: &Node) -> bool {
        self.root() == *root
    }
}
