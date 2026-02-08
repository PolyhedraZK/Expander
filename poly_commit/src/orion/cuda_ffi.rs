//! Raw FFI bindings to CUDA Orion PCS kernels.

#[cfg(feature = "cuda")]
extern "C" {
    /// GPU-accelerated matrix transpose for M31 elements.
    ///
    /// Transposes a rows x cols matrix stored in row-major order.
    /// Each element is a u32 (M31 field element or packed field element component).
    ///
    /// Returns 0 on success.
    pub fn cuda_m31_transpose(d_input: *const u32, d_output: *mut u32, rows: u32, cols: u32)
        -> i32;
}
