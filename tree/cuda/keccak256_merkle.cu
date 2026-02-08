// CUDA kernels for Keccak-256 Merkle tree construction
//
// Builds the tree bottom-up, one level at a time. Each level is
// embarrassingly parallel: each thread computes one node hash.
//
// Leaf hashing:  Keccak-256(64-byte leaf data) -> 32-byte hash
// Node hashing:  Keccak-256(left_hash || right_hash) -> 32-byte hash
// Both are exactly 64-byte inputs, matching the single-block Keccak optimization.

#include "keccak256.cuh"
#include <stdio.h>

#define HASH_BYTES 32
#define LEAF_DATA_BYTES 64

// ============================================================================
// Leaf hashing kernel: hash all leaves in parallel
// ============================================================================

// Input:  d_leaves[num_leaves * 64] - raw leaf data
// Output: d_leaf_hashes[num_leaves * 32] - Keccak-256 hashes
__global__ void keccak256_leaf_hash_kernel(
    const uint8_t* __restrict__ d_leaves,
    uint8_t* __restrict__ d_leaf_hashes,
    uint32_t num_leaves
) {
    uint32_t idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= num_leaves) return;

    const uint8_t* leaf_data = d_leaves + idx * LEAF_DATA_BYTES;
    uint8_t* hash_out = d_leaf_hashes + idx * HASH_BYTES;

    keccak256_64bytes(leaf_data, hash_out);
}

// ============================================================================
// Node hashing kernel: hash one level of internal nodes
// ============================================================================

// For a given level with `num_nodes` nodes:
//   node[i] = Keccak-256(child[2*i] || child[2*i+1])
//
// d_children: hash array of the child level (2 * num_nodes * 32 bytes)
// d_parents:  hash array for this level (num_nodes * 32 bytes)
__global__ void keccak256_node_hash_kernel(
    const uint8_t* __restrict__ d_children,
    uint8_t* __restrict__ d_parents,
    uint32_t num_nodes
) {
    uint32_t idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= num_nodes) return;

    // Concatenate left and right child hashes (32 + 32 = 64 bytes)
    uint8_t input[64];
    const uint8_t* left = d_children + (2 * idx) * HASH_BYTES;
    const uint8_t* right = d_children + (2 * idx + 1) * HASH_BYTES;

    for (int i = 0; i < HASH_BYTES; i++) {
        input[i] = left[i];
        input[HASH_BYTES + i] = right[i];
    }

    uint8_t* hash_out = d_parents + idx * HASH_BYTES;
    keccak256_64bytes(input, hash_out);
}

// ============================================================================
// Host orchestrator: build complete Merkle tree on GPU
// ============================================================================

#define MERKLE_BLOCK_SIZE 256

extern "C" {

/// Build a Merkle tree from pre-hashed leaf nodes on GPU.
///
/// Parameters:
///   d_leaf_hashes:  Device pointer to leaf hashes (num_leaves * 32 bytes)
///   d_tree_nodes:   Device pointer to output non-leaf nodes
///                   Layout: [(num_leaves-1) * 32 bytes], indexed same as CPU version:
///                   nodes[0] = root, nodes[left_child(0)] = root's left child, etc.
///   num_leaves:     Number of leaves (must be power of 2)
///   tree_height:    Height of tree (log2(num_leaves) + 1)
///
/// Returns 0 on success.
int cuda_keccak256_build_merkle_tree(
    const uint8_t* d_leaf_hashes,
    uint8_t* d_tree_nodes,
    uint32_t num_leaves,
    uint32_t tree_height
) {
    if (num_leaves == 0 || tree_height < 2) return -1;

    // The tree is stored with the same indexing as the CPU version:
    // non_leaf_nodes has (num_leaves - 1) entries
    // Level 0 = root (1 node), Level 1 = 2 nodes, ..., Level (h-2) = num_leaves/2 nodes
    //
    // We build bottom-up:
    // 1. Bottom internal level: hash pairs of leaf nodes
    // 2. Each subsequent level: hash pairs of child internal nodes

    cudaError_t err;

    // We need a temporary buffer for the bottom-most internal level
    // (parents of leaves). We build the tree into d_tree_nodes directly.
    //
    // The CPU code uses a flat array where:
    //   level k starts at index: sum_{j=0}^{k-1} 2^j = 2^k - 1
    //   level k has 2^k nodes (k=0 is root with 1 node)
    //
    // Levels from root: level 0 has 1 node, level 1 has 2 nodes, ...
    // level (tree_height-2) has num_leaves/2 nodes (parents of leaves)

    // Bottom internal level (parents of leaves)
    uint32_t bottom_level = tree_height - 2;
    uint32_t bottom_start = (1u << bottom_level) - 1;  // index in non_leaf_nodes
    uint32_t bottom_count = 1u << bottom_level;  // num_leaves / 2

    {
        int block_size = MERKLE_BLOCK_SIZE;
        int num_blocks = (bottom_count + block_size - 1) / block_size;

        keccak256_node_hash_kernel<<<num_blocks, block_size>>>(
            d_leaf_hashes,
            d_tree_nodes + bottom_start * HASH_BYTES,
            bottom_count
        );

        err = cudaGetLastError();
        if (err != cudaSuccess) return (int)err;
    }

    // Build upper levels
    for (int level = (int)bottom_level - 1; level >= 0; level--) {
        uint32_t level_start = (1u << level) - 1;
        uint32_t level_count = 1u << level;
        uint32_t child_start = (1u << (level + 1)) - 1;

        int block_size = MERKLE_BLOCK_SIZE;
        int num_blocks = (level_count + block_size - 1) / block_size;

        keccak256_node_hash_kernel<<<num_blocks, block_size>>>(
            d_tree_nodes + child_start * HASH_BYTES,
            d_tree_nodes + level_start * HASH_BYTES,
            level_count
        );

        err = cudaGetLastError();
        if (err != cudaSuccess) return (int)err;
    }

    err = cudaDeviceSynchronize();
    return (err == cudaSuccess) ? 0 : (int)err;
}

/// Hash raw leaves into leaf hashes on GPU.
///
/// d_raw_leaves:   num_leaves * 64 bytes of leaf data
/// d_leaf_hashes:  output: num_leaves * 32 bytes of Keccak-256 hashes
int cuda_keccak256_hash_leaves(
    const uint8_t* d_raw_leaves,
    uint8_t* d_leaf_hashes,
    uint32_t num_leaves
) {
    if (num_leaves == 0) return 0;

    int block_size = MERKLE_BLOCK_SIZE;
    int num_blocks = (num_leaves + block_size - 1) / block_size;

    keccak256_leaf_hash_kernel<<<num_blocks, block_size>>>(
        d_raw_leaves, d_leaf_hashes, num_leaves
    );

    cudaError_t err = cudaGetLastError();
    if (err != cudaSuccess) return (int)err;

    err = cudaDeviceSynchronize();
    return (err == cudaSuccess) ? 0 : (int)err;
}

} // extern "C"
