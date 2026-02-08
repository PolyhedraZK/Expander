//! Safe wrapper for GPU-accelerated matrix transpose in Orion PCS commit.
//!
//! The transpose in `commit_encoded` is memory-bandwidth-bound and benefits
//! significantly from GPU's high memory bandwidth (typically 10-20x faster
//! than CPU for large matrices).

#[cfg(feature = "cuda")]
use crate::orion::cuda_ffi;

/// Minimum matrix size (total elements) to dispatch to GPU.
#[cfg(feature = "cuda")]
const GPU_TRANSPOSE_THRESHOLD: usize = 4096;

/// Try to transpose a matrix on GPU.
///
/// Takes a row-major matrix of u32 elements (rows x cols) and writes
/// the transposed result (cols x rows) to the output slice.
///
/// Returns `true` if GPU transpose succeeded, `false` if unavailable or errored
/// (caller should fall back to CPU).
#[cfg(feature = "cuda")]
pub fn try_gpu_transpose(input: &[u32], output: &mut [u32], rows: usize, cols: usize) -> bool {
    let total = rows * cols;
    if total < GPU_TRANSPOSE_THRESHOLD {
        return false;
    }

    assert_eq!(input.len(), total);
    assert_eq!(output.len(), total);

    unsafe {
        // Allocate device memory
        let mut d_input: *mut u32 = std::ptr::null_mut();
        let mut d_output: *mut u32 = std::ptr::null_mut();
        let byte_size = total * std::mem::size_of::<u32>();

        if cuda_malloc(&mut d_input, byte_size) != 0 {
            return false;
        }
        if cuda_malloc(&mut d_output, byte_size) != 0 {
            cuda_free(d_input as *mut std::ffi::c_void);
            return false;
        }

        // Copy input to device
        if cuda_memcpy_h2d(
            d_input as *mut std::ffi::c_void,
            input.as_ptr() as *const std::ffi::c_void,
            byte_size,
        ) != 0
        {
            cuda_free(d_input as *mut std::ffi::c_void);
            cuda_free(d_output as *mut std::ffi::c_void);
            return false;
        }

        // Run transpose
        let err = cuda_ffi::cuda_m31_transpose(d_input, d_output, rows as u32, cols as u32);

        if err != 0 {
            cuda_free(d_input as *mut std::ffi::c_void);
            cuda_free(d_output as *mut std::ffi::c_void);
            return false;
        }

        // Copy result back
        let copy_err = cuda_memcpy_d2h(
            output.as_mut_ptr() as *mut std::ffi::c_void,
            d_output as *const std::ffi::c_void,
            byte_size,
        );

        cuda_free(d_input as *mut std::ffi::c_void);
        cuda_free(d_output as *mut std::ffi::c_void);

        copy_err == 0
    }
}

/// Stub for non-CUDA builds.
#[cfg(not(feature = "cuda"))]
pub fn try_gpu_transpose(_input: &[u32], _output: &mut [u32], _rows: usize, _cols: usize) -> bool {
    false
}

// Minimal CUDA runtime bindings
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
