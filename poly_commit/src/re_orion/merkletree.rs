use arith::Field;
use gkr_hashers::FiatShamirHasher;

pub struct MerkleTree<H: FiatShamirHasher> {
    node: Vec<u8>,
    hasher: H,
    pub height: usize,
}

impl<H: FiatShamirHasher> MerkleTree<H> {
    pub fn build_inplace(node: &mut [u8], hasher: &H, height: usize) {
        let block_size = H::DIGEST_SIZE;
        let n = 1 << height;
        for i in (1..n).rev() {
            let u = i * block_size;
            let v = i * 2 * block_size;
            let w = v + block_size;
            for j in 0..block_size {
                node[u + j] = node[v + j] ^ node[w + j];
            }
// println!("<{}> -- {:?}", i, &node[u..u + block_size]);
// println!("{:?} + {:?}", &node[v..v + block_size], &node[w..w + block_size]);
            hasher.hash_inplace(&mut node[u..u + block_size]);
// println!("-> {:?}", &node[u..u + block_size]);
        }
    }

    pub fn build<F: Field>(src: &[F], node: &mut [u8], hasher: &H) -> usize {
        let n = src.len();
        let mut h = 0;
        while (1 << h) < n {
            h += 1;
        }
        let block_size = H::DIGEST_SIZE;
        assert!(node.len() >= n * 2 * block_size);
        let leaves = &mut node[(1 << h) * block_size..(1 << (h + 1)) * block_size];
        let mut f = vec![0u8; F::SIZE];
        leaves.fill(0);
        for (i, si) in src.iter().enumerate().take(n) {
            let leaf = &mut leaves[i * block_size..(i + 1) * block_size];
             si.to_bytes(&mut f);
// println!("<{}> {:?}", i, &f);
             hasher.hash(leaf, &f);
// println!("-> {:?}", &leaf);
        }
// println!("prepared node {:?}", node);
        Self::build_inplace(node, hasher, h);
        h
    }

    // path from leaf to root
    pub fn verify(root: &[u8], leaf: &mut [u8], path: &[u8], hasher: &H) -> bool {
        assert!(path.len() % H::DIGEST_SIZE == 0);
        // hasher.hash_inplace(leaf);
// println!("{:?} ->", leaf);
        for i in (0..path.len()).step_by(H::DIGEST_SIZE) {
            let bro = &path[i..i + H::DIGEST_SIZE];
            for j in 0..H::DIGEST_SIZE {
                leaf[j] ^= bro[j];
            }
            hasher.hash_inplace(leaf);
// println!("{:?}", leaf);
        }
// println!("? {:?}", root);
        for i in 0..H::DIGEST_SIZE {
            if leaf[i] != root[i] {
                return false;
            }
        }
        true
    }

    fn prove(node: &[u8], leaf: usize, root: usize) -> Vec<u8> {
        let block_size = H::DIGEST_SIZE;
        let mut res = vec![];
        let mut u = leaf;
        while u > root {
            let v = u ^ 1;
// println!("{} - {:?}", u, &node[v * block_size..(v + 1) * block_size]);
            res.extend_from_slice(&node[v * block_size..(v + 1) * block_size]);
            u >>= 1;
        }
        res
    }

    pub fn new(n: usize) -> Self {
        let mut h = 0;
        while (1 << h) < n {
            h += 1;
        }
        h += 1;
        MerkleTree {
            node: vec![0u8; (1 << h) * H::DIGEST_SIZE],
            hasher: H::new(),
            height: 0,
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.node.fill(0);
    }

    #[inline(always)]
    pub fn commit(&self) -> Vec<u8> {
        self.get_node(1)
    }

    #[inline(always)]
    pub fn get_node(&self, node: usize) -> Vec<u8>{
        self.node[node * H::DIGEST_SIZE..(node + 1) * H::DIGEST_SIZE].to_vec()
    }

    pub fn print_tree(&self, node: usize) {
        if node > (1 << (self.height + 1)) {
            return;
        }
        println!("<{}> {:?}", node, &self.node[node * H::DIGEST_SIZE..(node + 1) * H::DIGEST_SIZE]);
        self.print_tree(node * 2);
        self.print_tree(node * 2 + 1);
    }
}

pub trait MerkleTreeAPI {
    fn build<F: Field>(&mut self, src: &[F]) -> usize;
    fn prove(&self, leaf: usize, root: usize) -> Vec<u8>;
    fn verify(&self, leaf: &mut [u8], path: &[u8]) -> bool;
}

impl<H: FiatShamirHasher> MerkleTreeAPI for MerkleTree<H> {

    #[inline(always)]
    fn build<F: Field>(&mut self, src: &[F]) -> usize {
        self.height = Self::build(src, &mut self.node, &self.hasher);
        self.height
    }

    #[inline(always)]
    fn prove(&self, leaf: usize, root: usize) -> Vec<u8> {
        Self::prove(&self.node, leaf, root)
    }

    #[inline(always)]
    fn verify(&self, leaf: &mut [u8], path: &[u8]) -> bool {
        Self::verify(&self.node[H::DIGEST_SIZE..H::DIGEST_SIZE * 2], leaf, path, &self.hasher)
    }
}