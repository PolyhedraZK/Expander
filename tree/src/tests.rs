use arith::Field;
use ark_std::{rand::RngCore, test_rng};
use babybear::BabyBearx16;
use poseidon::PoseidonBabyBearParams;

use crate::{Leaf, Tree};

#[test]
fn test_tree() {
    // Initialize a random number generator for the test
    let mut rng = test_rng();

    // Create a new instance of PoseidonBabyBearParams for hashing
    let leaf_hasher = PoseidonBabyBearParams::new(&mut rng);

    // Test trees of different heights, from 4 to 14
    for height in 4..15 {
        // Generate random leaves for the tree
        // The number of leaves is 2^(height-1)
        let leaves: Vec<Leaf> = (0..(1 << (height - 1)))
            .map(|_| BabyBearx16::random_unsafe(&mut rng).into())
            .collect();

        // Create a new tree with the generated leaves
        let tree = Tree::new_with_leaves(&leaf_hasher, leaves, height);

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

            // Verify the proof
            // This checks that the leaf at the given index is indeed part of the tree
            // with the given root, using the generated proof
            assert!(proof.verify(&root, &leaf_hasher));
        }
    }
}
