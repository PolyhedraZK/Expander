//! Safe wrapper for GPU-accelerated Merkle tree construction.
//!
//! Provides `gpu_build_merkle_tree` which builds the tree on GPU and returns
//! the non-leaf nodes as `Vec<Node>`, matching the CPU `new_with_leaf_nodes` API.

#[cfg(feature = "cuda")]
use crate::cuda_ffi;
#[cfg(feature = "cuda")]
use crate::LEAF_HASH_BYTES;
use crate::Node;

/// Minimum number of leaves to dispatch to GPU.
/// Below this, CPU is faster due to kernel launch + PCIe overhead.
#[cfg(feature = "cuda")]
const GPU_TREE_THRESHOLD: usize = 256;

/// Try to build a Merkle tree on GPU from pre-hashed leaf nodes.
///
/// Returns `Some(non_leaf_nodes)` on success, `None` if GPU is unavailable,
/// the input is too small, or a CUDA error occurs.
///
/// The returned Vec has the same layout as the CPU `new_with_leaf_nodes`:
/// nodes[0] = root, using left_child_index(i) = 2*i+1.
#[cfg(feature = "cuda")]
pub fn gpu_build_merkle_tree(leaf_nodes: &[Node], tree_height: u32) -> Option<Vec<Node>> {
    let num_leaves = leaf_nodes.len();

    if num_leaves < GPU_TREE_THRESHOLD {
        return None;
    }

    if !num_leaves.is_power_of_two() || tree_height < 2 {
        return None;
    }

    let num_non_leaf = num_leaves - 1;

    unsafe {
        // Allocate device memory for leaf hashes
        let leaf_bytes = num_leaves * LEAF_HASH_BYTES;
        let mut d_leaf_hashes: *mut u8 = std::ptr::null_mut();
        if cuda_malloc(
            &mut d_leaf_hashes as *mut *mut u8 as *mut *mut std::ffi::c_void,
            leaf_bytes,
        ) != 0
        {
            return None;
        }

        // Copy leaf hashes to device
        if cuda_memcpy_h2d(
            d_leaf_hashes as *mut std::ffi::c_void,
            leaf_nodes.as_ptr() as *const std::ffi::c_void,
            leaf_bytes,
        ) != 0
        {
            cuda_free(d_leaf_hashes as *mut std::ffi::c_void);
            return None;
        }

        // Allocate device memory for non-leaf nodes
        let tree_bytes = num_non_leaf * LEAF_HASH_BYTES;
        let mut d_tree_nodes: *mut u8 = std::ptr::null_mut();
        if cuda_malloc(
            &mut d_tree_nodes as *mut *mut u8 as *mut *mut std::ffi::c_void,
            tree_bytes,
        ) != 0
        {
            cuda_free(d_leaf_hashes as *mut std::ffi::c_void);
            return None;
        }

        // Build tree on GPU
        let err = cuda_ffi::cuda_keccak256_build_merkle_tree(
            d_leaf_hashes,
            d_tree_nodes,
            num_leaves as u32,
            tree_height,
        );

        if err != 0 {
            cuda_free(d_leaf_hashes as *mut std::ffi::c_void);
            cuda_free(d_tree_nodes as *mut std::ffi::c_void);
            return None;
        }

        // Copy results back to host
        let mut non_leaf_nodes = vec![Node::default(); num_non_leaf];
        let copy_err = cuda_memcpy_d2h(
            non_leaf_nodes.as_mut_ptr() as *mut std::ffi::c_void,
            d_tree_nodes as *const std::ffi::c_void,
            tree_bytes,
        );

        cuda_free(d_leaf_hashes as *mut std::ffi::c_void);
        cuda_free(d_tree_nodes as *mut std::ffi::c_void);

        if copy_err != 0 {
            return None;
        }

        Some(non_leaf_nodes)
    }
}

/// Stub for non-CUDA builds.
#[cfg(not(feature = "cuda"))]
pub fn gpu_build_merkle_tree(_leaf_nodes: &[Node], _tree_height: u32) -> Option<Vec<Node>> {
    None
}

// Minimal CUDA runtime bindings
#[cfg(feature = "cuda")]
extern "C" {
    #[link_name = "cudaMalloc"]
    fn cuda_malloc(devptr: *mut *mut std::ffi::c_void, size: usize) -> i32;

    #[link_name = "cudaFree"]
    fn cuda_free(devptr: *mut std::ffi::c_void) -> i32;

    #[link_name = "cudaMemcpy"]
    fn cuda_memcpy_raw(
        dst: *mut std::ffi::c_void,
        src: *const std::ffi::c_void,
        count: usize,
        kind: i32,
    ) -> i32;
}

#[cfg(feature = "cuda")]
const CUDA_MEMCPY_HOST_TO_DEVICE: i32 = 1;
#[cfg(feature = "cuda")]
const CUDA_MEMCPY_DEVICE_TO_HOST: i32 = 2;

#[cfg(feature = "cuda")]
unsafe fn cuda_memcpy_h2d(
    dst: *mut std::ffi::c_void,
    src: *const std::ffi::c_void,
    count: usize,
) -> i32 {
    cuda_memcpy_raw(dst, src, count, CUDA_MEMCPY_HOST_TO_DEVICE)
}

#[cfg(feature = "cuda")]
unsafe fn cuda_memcpy_d2h(
    dst: *mut std::ffi::c_void,
    src: *const std::ffi::c_void,
    count: usize,
) -> i32 {
    cuda_memcpy_raw(dst, src, count, CUDA_MEMCPY_DEVICE_TO_HOST)
}
