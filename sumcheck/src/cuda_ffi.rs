//! Raw FFI bindings to the CUDA sumcheck kernels.
//!
//! These functions are only available when compiled with the `cuda` feature
//! and nvcc is detected during build.

#[cfg(feature = "cuda")]
extern "C" {
    /// GPU-accelerated poly_eval_at for M31ext3 fields.
    ///
    /// Computes [p0, p1, p2] where:
    ///   p0 = sum_i bk_hg[2i]   * bk_f[2i]
    ///   p1 = sum_i bk_hg[2i+1] * bk_f[2i+1]
    ///   p2 = sum_i (bk_hg[2i] + bk_hg[2i+1]) * (bk_f[2i] + bk_f[2i+1])
    ///
    /// All pointers are device memory. Each M31ext3 element is 3 x u32.
    /// d_bk_f, d_bk_hg: 2*eval_size elements = 6*eval_size u32
    /// d_result: 3 elements = 9 u32 [p0, p1, p2]
    ///
    /// Returns 0 on success.
    pub fn cuda_m31ext3_poly_eval(
        d_bk_f: *const u32,
        d_bk_hg: *const u32,
        d_result: *mut u32,
        eval_size: u32,
    ) -> i32;

    /// GPU-accelerated receive_challenge for M31ext3 fields.
    ///
    /// Updates bookkeeping arrays after receiving challenge r:
    ///   bk_f[i]  = bk_f[2i]  + (bk_f[2i+1]  - bk_f[2i])  * r
    ///   bk_hg[i] = bk_hg[2i] + (bk_hg[2i+1] - bk_hg[2i]) * r
    ///
    /// When first_round != 0, reads from d_init_v (M31 base, 1 u32 each)
    /// instead of d_bk_f for the f bookkeeping.
    ///
    /// Returns 0 on success.
    pub fn cuda_m31ext3_receive_challenge(
        d_bk_f: *mut u32,
        d_bk_hg: *mut u32,
        d_challenge_r: *const u32,
        eval_size: u32,
        first_round: i32,
        d_init_v: *const u32,
    ) -> i32;
}
