// CUDA kernel declarations for M31 sumcheck operations
// These kernels accelerate the hot loops in SumcheckProductGateHelper

#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// poly_eval_at kernel: parallel reduction computing [p0, p1, p2]
//
// For each i in [0, eval_size):
//   p0 += bk_hg[2i]   * bk_f[2i]
//   p1 += bk_hg[2i+1] * bk_f[2i+1]
//   p2 += (bk_hg[2i] + bk_hg[2i+1]) * (bk_f[2i] + bk_f[2i+1])
//
// Works with M31ext3 field elements (3 x uint32_t each).
//
// Parameters:
//   d_bk_f:    device pointer to bk_f array (eval_size*2 M31ext3 elements = 6*eval_size uint32_t)
//   d_bk_hg:   device pointer to bk_hg array (eval_size*2 M31ext3 elements)
//   d_result:  device pointer to output (3 M31ext3 elements = 9 uint32_t)
//   eval_size: number of pairs to process
//
// Returns 0 on success, non-zero on CUDA error.
int cuda_m31ext3_poly_eval(
    const uint32_t* d_bk_f,
    const uint32_t* d_bk_hg,
    uint32_t* d_result,
    uint32_t eval_size
);

// ============================================================================
// receive_challenge kernel: bookkeeping update after receiving challenge r
//
// For each i in [0, eval_size):
//   bk_f[i]  = bk_f[2i]  + (bk_f[2i+1]  - bk_f[2i])  * r
//   bk_hg[i] = bk_hg[2i] + (bk_hg[2i+1] - bk_hg[2i]) * r
//
// Parameters:
//   d_bk_f:        device pointer to bk_f (read from [0..2*eval_size), write to [0..eval_size))
//   d_bk_hg:       device pointer to bk_hg (same layout)
//   d_challenge_r: device pointer to the challenge value (1 M31ext3 = 3 uint32_t)
//   eval_size:     number of output elements
//   first_round:   if nonzero, read from d_init_v instead of d_bk_f
//   d_init_v:      device pointer to init_v (only used when first_round != 0)
//                  init_v elements are M31 base field (1 uint32_t each),
//                  while bk_f/bk_hg are M31ext3 (3 uint32_t each)
//
// Returns 0 on success, non-zero on CUDA error.
int cuda_m31ext3_receive_challenge(
    uint32_t* d_bk_f,
    uint32_t* d_bk_hg,
    const uint32_t* d_challenge_r,
    uint32_t eval_size,
    int first_round,
    const uint32_t* d_init_v
);

#ifdef __cplusplus
}
#endif
