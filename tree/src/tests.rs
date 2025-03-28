use std::io::Cursor;

use ark_std::{rand::RngCore, test_rng};
use serdes::ExpSerde;

use crate::{Leaf, Path, Tree};

fn random_leaf<R: RngCore>(rng: &mut R) -> Leaf {
    Leaf::new({
        let mut data = [0u8; 64];
        rng.fill_bytes(&mut data);
        data
    })
}

#[test]
fn test_tree() {
    // Initialize a random number generator for the test
    let mut rng = test_rng();

    // Create a new instance of PoseidonBabyBearParams for hashing
    // let leaf_hasher = PoseidonBabyBearParams::new(&mut rng);

    // Test trees of different heights, from 4 to 14
    for height in 4..15 {
        // Generate random leaves for the tree
        // The number of leaves is 2^(height-1)
        let leaves: Vec<Leaf> = (0..(1 << (height - 1)))
            .map(|_| random_leaf(&mut rng))
            .collect();

        // Create a new tree with the generated leaves
        let tree = Tree::new_with_leaves(leaves);

        // Perform 100 random verifications for each tree
        for _ in 0..100 {
            // Select a random leaf index
            let index = rng.next_u32() % (1 << (height - 1));

            // Generate a proof for the selected leaf
            let proof = tree.gen_proof(index as usize, height);

            // Get the root of the tree
            let root = tree.root();

            // Print debug information
            println!("index: {}\n", index);
            println!("root: {}\n", root);
            println!("tree {}\n", tree);
            println!("path {}\n", proof);

            // Serialize and deserialize the proof
            let mut buffer: Vec<u8> = Vec::new();
            proof.serialize_into(&mut buffer).unwrap();
            let mut cursor = Cursor::new(buffer);
            let deserialized_proof = Path::deserialize_from(&mut cursor).unwrap();

            // Verify the proof
            // This checks that the leaf at the given index is indeed part of the tree
            // with the given root, using the generated proof
            assert!(deserialized_proof.verify(&root));
        }
    }
}

#[test]
fn test_path_prefix() {
    const MAX_LEAVES: usize = 1 << 15;

    let mut rng = test_rng();
    let leaves_buffer: Vec<_> = (0..MAX_LEAVES).map(|_| random_leaf(&mut rng)).collect();

    let whole_tree = Tree::new_with_leaves(leaves_buffer.clone());

    for prefix_len in 1..=6 {
        let sub_tree_num = 1 << prefix_len;
        let sub_tree_leaves = 1 << (MAX_LEAVES.ilog2() - prefix_len);

        let sub_trees: Vec<Tree> = leaves_buffer
            .chunks(sub_tree_leaves)
            .map(|c| Tree::new_with_leaves(c.to_vec()))
            .collect();

        // Check vanilla path prefixing
        for _ in 0..100 {
            let sub_tree_index = rng.next_u64() as usize % sub_tree_num;
            let sub_tree_opening_at = rng.next_u64() as usize % sub_tree_leaves;
            let sub_tree_leaves_starts = sub_tree_index * sub_tree_leaves;
            let whole_tree_opening_at = sub_tree_leaves_starts + sub_tree_opening_at;

            let whole_tree_proof =
                whole_tree.gen_proof(whole_tree_opening_at, MAX_LEAVES.ilog2() as usize + 1);
            assert!(whole_tree_proof.verify(&whole_tree.root()));

            let mut sub_proof = sub_trees[sub_tree_index]
                .gen_proof(sub_tree_opening_at, sub_tree_leaves.ilog2() as usize + 1);
            assert!(sub_proof.verify(&sub_trees[sub_tree_index].root()));

            assert_eq!(whole_tree_proof.leaf, sub_proof.leaf);

            let prefix = whole_tree_proof.path_nodes[0..prefix_len as usize].to_vec();

            sub_proof.prefix_with(&prefix, sub_tree_index);

            assert!(sub_proof.verify(&whole_tree.root()));
        }

        // Check range opening prefixing
        let range_opening_size = 1 << prefix_len;
        for _ in 0..100 {
            let sub_tree_index = rng.next_u64() as usize % sub_tree_num;
            let sub_tree_opening_at = {
                let random_index = rng.next_u64() as usize % sub_tree_leaves;
                (random_index / range_opening_size) * range_opening_size
            };
            let sub_tree_leaves_starts = sub_tree_index * sub_tree_leaves;
            let whole_tree_opening_at = sub_tree_leaves_starts + sub_tree_opening_at;

            let whole_tree_proof = whole_tree.gen_range_proof(
                whole_tree_opening_at,
                whole_tree_opening_at + range_opening_size - 1,
                MAX_LEAVES.ilog2() as usize + 1,
            );
            assert!(whole_tree_proof.verify(&whole_tree.root()));

            let mut sub_proof = sub_trees[sub_tree_index].gen_range_proof(
                sub_tree_opening_at,
                sub_tree_opening_at + range_opening_size - 1,
                sub_tree_leaves.ilog2() as usize + 1,
            );
            assert!(sub_proof.verify(&sub_trees[sub_tree_index].root()));

            assert_eq!(whole_tree_proof.leaves, sub_proof.leaves);

            let prefix = whole_tree_proof.path_nodes[0..prefix_len as usize].to_vec();

            sub_proof.prefix_with(&prefix, sub_tree_index);

            assert!(sub_proof.verify(&whole_tree.root()));
        }
    }
}
