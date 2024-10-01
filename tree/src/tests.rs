use arith::Field;
use ark_std::{rand::RngCore, test_rng};
use babybear::BabyBearx16;
use poseidon::PoseidonBabyBearParams;

use crate::{Leaf, Tree};

#[test]
fn test_tree() {
    let mut rng = test_rng();
    let hasher = PoseidonBabyBearParams::new(&mut rng);

    for height in 4..15 {
        let leaves: Vec<Leaf> = (0..(1 << (height - 1)))
            .map(|_| BabyBearx16::random_unsafe(&mut rng).into())
            .collect();
        let tree = Tree::new_with_leaves(&hasher, leaves, height);

        for _ in 0..100 {
            let index = rng.next_u32() % (1 << (height - 1));
            let proof = tree.gen_proof(index as usize, height);
            let root = tree.root();

            println!("index: {}\n", index);
            println!("root: {}\n", root);
            println!("tree {}\n", tree);
            println!("path {}\n", proof);

            assert!(proof.verify(&root, &tree.leaves[index as usize], &hasher));
        }
    }
}
