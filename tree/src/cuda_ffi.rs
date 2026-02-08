//! Raw FFI bindings to CUDA Keccak-256 Merkle tree kernels.

#[cfg(feature = "cuda")]
extern "C" {
    /// Build a Merkle tree from pre-hashed leaf nodes on GPU.
    ///
    /// d_leaf_hashes:  Device pointer to leaf hashes (num_leaves * 32 bytes)
    /// d_tree_nodes:   Device pointer to output non-leaf nodes ((num_leaves-1) * 32 bytes)
    /// num_leaves:     Number of leaves (must be power of 2)
    /// tree_height:    log2(num_leaves) + 1
    ///
    /// Returns 0 on success.
    pub fn cuda_keccak256_build_merkle_tree(
        d_leaf_hashes: *const u8,
        d_tree_nodes: *mut u8,
        num_leaves: u32,
        tree_height: u32,
    ) -> i32;

    /// Hash raw 64-byte leaves into 32-byte Keccak-256 hashes on GPU.
    ///
    /// d_raw_leaves:   Device pointer (num_leaves * 64 bytes)
    /// d_leaf_hashes:  Device output (num_leaves * 32 bytes)
    ///
    /// Returns 0 on success.
    pub fn cuda_keccak256_hash_leaves(
        d_raw_leaves: *const u8,
        d_leaf_hashes: *mut u8,
        num_leaves: u32,
    ) -> i32;
}
