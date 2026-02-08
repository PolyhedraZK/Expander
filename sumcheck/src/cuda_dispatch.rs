//! Safe wrappers around CUDA sumcheck kernels with threshold checks and CPU fallback.
//!
//! The GPU dispatch is transparent to the caller: if CUDA is not available or the
//! input is too small, the functions return `None` and the caller falls through to
//! the existing CPU implementation.

#[cfg(feature = "cuda")]
use crate::cuda_ffi;

/// Minimum eval_size to dispatch to GPU. Below this threshold, CPU is faster
/// due to kernel launch overhead and PCIe transfer costs.
const GPU_DISPATCH_THRESHOLD: usize = 1024;

/// Result of a GPU poly_eval_at call.
/// Contains [p0, p1, p2] as raw u32 limbs (3 limbs per M31ext3 element = 9 u32 total).
#[cfg(feature = "cuda")]
pub struct GpuPolyEvalResult {
    pub p0_limbs: [u32; 3],
    pub p1_limbs: [u32; 3],
    pub p2_limbs: [u32; 3],
}

/// Try to run poly_eval_at on GPU. Returns None if CUDA is unavailable,
/// eval_size is below threshold, or an error occurs (in which case the
/// caller should fall back to CPU).
///
/// # Safety
/// All device pointers must be valid CUDA device memory of the correct size.
#[cfg(feature = "cuda")]
pub unsafe fn try_gpu_poly_eval(
    d_bk_f: *const u32,
    d_bk_hg: *const u32,
    eval_size: usize,
) -> Option<GpuPolyEvalResult> {
    if eval_size < GPU_DISPATCH_THRESHOLD {
        return None;
    }

    // Allocate device memory for result (9 u32)
    let mut d_result: *mut u32 = std::ptr::null_mut();
    let alloc_err = cuda_malloc(&mut d_result, 9 * std::mem::size_of::<u32>());
    if alloc_err != 0 || d_result.is_null() {
        return None;
    }

    let err = cuda_ffi::cuda_m31ext3_poly_eval(d_bk_f, d_bk_hg, d_result, eval_size as u32);

    if err != 0 {
        cuda_free(d_result as *mut std::ffi::c_void);
        return None;
    }

    // Copy result back to host
    let mut result = [0u32; 9];
    let copy_err = cuda_memcpy_d2h(
        result.as_mut_ptr() as *mut std::ffi::c_void,
        d_result as *const std::ffi::c_void,
        9 * std::mem::size_of::<u32>(),
    );

    cuda_free(d_result as *mut std::ffi::c_void);

    if copy_err != 0 {
        return None;
    }

    Some(GpuPolyEvalResult {
        p0_limbs: [result[0], result[1], result[2]],
        p1_limbs: [result[3], result[4], result[5]],
        p2_limbs: [result[6], result[7], result[8]],
    })
}

/// Try to run receive_challenge on GPU.
///
/// # Safety
/// All device pointers must be valid CUDA device memory.
#[cfg(feature = "cuda")]
pub unsafe fn try_gpu_receive_challenge(
    d_bk_f: *mut u32,
    d_bk_hg: *mut u32,
    d_challenge_r: *const u32,
    eval_size: usize,
    first_round: bool,
    d_init_v: *const u32,
) -> bool {
    if eval_size < GPU_DISPATCH_THRESHOLD {
        return false;
    }

    let err = cuda_ffi::cuda_m31ext3_receive_challenge(
        d_bk_f,
        d_bk_hg,
        d_challenge_r,
        eval_size as u32,
        if first_round { 1 } else { 0 },
        d_init_v,
    );

    err == 0
}

// Minimal CUDA runtime bindings for memory management
#[cfg(feature = "cuda")]
extern "C" {
    #[link_name = "cudaMalloc"]
    fn cuda_malloc(devptr: *mut *mut u32, size: usize) -> i32;

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
const CUDA_MEMCPY_DEVICE_TO_HOST: i32 = 2;

#[cfg(feature = "cuda")]
unsafe fn cuda_memcpy_d2h(
    dst: *mut std::ffi::c_void,
    src: *const std::ffi::c_void,
    count: usize,
) -> i32 {
    cuda_memcpy_raw(dst, src, count, CUDA_MEMCPY_DEVICE_TO_HOST)
}
