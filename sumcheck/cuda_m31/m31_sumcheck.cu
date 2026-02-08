// CUDA kernels for M31ext3 sumcheck operations
//
// Implements GPU-accelerated poly_eval_at (shared-memory parallel reduction)
// and receive_challenge (bookkeeping update) for the sumcheck protocol.
//
// These are the two hot loops in SumcheckProductGateHelper that dominate
// proving time for GKR proofs.

#include "m31_field_ops.cuh"
#include "m31_sumcheck.cuh"

#include <stdio.h>

// ============================================================================
// poly_eval_at: compute [p0, p1, p2] via shared-memory parallel reduction
// ============================================================================

// Block-level reduction kernel. Each block reduces a chunk of pairs into
// a partial [p0, p1, p2] stored in global memory.
__global__ void poly_eval_kernel_m31ext3(
    const uint32_t* __restrict__ d_bk_f,   // 2*eval_size M31ext3 elements (6*eval_size u32)
    const uint32_t* __restrict__ d_bk_hg,  // 2*eval_size M31ext3 elements
    uint32_t* __restrict__ d_block_results, // num_blocks * 9 u32 (3 M31ext3 per block)
    uint32_t eval_size
) {
    // Each M31ext3 is 3 u32s. Arrays are packed: element i starts at offset i*3
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int tid = threadIdx.x;

    // Shared memory: 3 M31ext3 per thread = 9 u32 per thread
    extern __shared__ uint32_t smem[];
    // Layout: [p0_limb0, p0_limb1, p0_limb2] for each thread,
    //   then [p1_limb0, ...], then [p2_limb0, ...]
    uint32_t* s_p0 = smem;                         // blockDim.x * 3
    uint32_t* s_p1 = smem + blockDim.x * 3;        // blockDim.x * 3
    uint32_t* s_p2 = smem + blockDim.x * 6;        // blockDim.x * 3

    // Initialize to zero
    for (int l = 0; l < 3; l++) {
        s_p0[tid * 3 + l] = 0;
        s_p1[tid * 3 + l] = 0;
        s_p2[tid * 3 + l] = 0;
    }

    if (idx < (int)eval_size) {
        // Load f_v_0 = bk_f[2*idx], f_v_1 = bk_f[2*idx+1]
        M31ext3 f_v_0, f_v_1;
        int base_f = idx * 6; // 2 * idx * 3
        f_v_0.v[0] = d_bk_f[base_f + 0];
        f_v_0.v[1] = d_bk_f[base_f + 1];
        f_v_0.v[2] = d_bk_f[base_f + 2];
        f_v_1.v[0] = d_bk_f[base_f + 3];
        f_v_1.v[1] = d_bk_f[base_f + 4];
        f_v_1.v[2] = d_bk_f[base_f + 5];

        // Load hg_v_0 = bk_hg[2*idx], hg_v_1 = bk_hg[2*idx+1]
        M31ext3 hg_v_0, hg_v_1;
        int base_hg = idx * 6;
        hg_v_0.v[0] = d_bk_hg[base_hg + 0];
        hg_v_0.v[1] = d_bk_hg[base_hg + 1];
        hg_v_0.v[2] = d_bk_hg[base_hg + 2];
        hg_v_1.v[0] = d_bk_hg[base_hg + 3];
        hg_v_1.v[1] = d_bk_hg[base_hg + 4];
        hg_v_1.v[2] = d_bk_hg[base_hg + 5];

        // p0 = hg_v_0 * f_v_0
        M31ext3 lp0 = m31ext3_mul(hg_v_0, f_v_0);
        // p1 = hg_v_1 * f_v_1
        M31ext3 lp1 = m31ext3_mul(hg_v_1, f_v_1);
        // p2 = (hg_v_0 + hg_v_1) * (f_v_0 + f_v_1)
        M31ext3 lp2 = m31ext3_mul(
            m31ext3_add(hg_v_0, hg_v_1),
            m31ext3_add(f_v_0, f_v_1)
        );

        for (int l = 0; l < 3; l++) {
            s_p0[tid * 3 + l] = lp0.v[l];
            s_p1[tid * 3 + l] = lp1.v[l];
            s_p2[tid * 3 + l] = lp2.v[l];
        }
    }

    __syncthreads();

    // Parallel reduction in shared memory
    for (int stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (tid < stride) {
            for (int l = 0; l < 3; l++) {
                s_p0[tid * 3 + l] = m31_add(s_p0[tid * 3 + l], s_p0[(tid + stride) * 3 + l]);
                s_p1[tid * 3 + l] = m31_add(s_p1[tid * 3 + l], s_p1[(tid + stride) * 3 + l]);
                s_p2[tid * 3 + l] = m31_add(s_p2[tid * 3 + l], s_p2[(tid + stride) * 3 + l]);
            }
        }
        __syncthreads();
    }

    // Thread 0 writes block result
    if (tid == 0) {
        int out_base = blockIdx.x * 9;
        for (int l = 0; l < 3; l++) {
            d_block_results[out_base + l]     = s_p0[l];
            d_block_results[out_base + 3 + l] = s_p1[l];
            d_block_results[out_base + 6 + l] = s_p2[l];
        }
    }
}

// Second-level reduction: reduce block partial results
__global__ void reduce_blocks_m31ext3(
    const uint32_t* __restrict__ d_src,
    uint32_t* __restrict__ d_dst,
    uint32_t num_blocks_to_reduce
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int tid = threadIdx.x;

    extern __shared__ uint32_t smem[];
    uint32_t* s_p0 = smem;
    uint32_t* s_p1 = smem + blockDim.x * 3;
    uint32_t* s_p2 = smem + blockDim.x * 6;

    for (int l = 0; l < 3; l++) {
        s_p0[tid * 3 + l] = 0;
        s_p1[tid * 3 + l] = 0;
        s_p2[tid * 3 + l] = 0;
    }

    if (idx < (int)num_blocks_to_reduce) {
        int base = idx * 9;
        for (int l = 0; l < 3; l++) {
            s_p0[tid * 3 + l] = d_src[base + l];
            s_p1[tid * 3 + l] = d_src[base + 3 + l];
            s_p2[tid * 3 + l] = d_src[base + 6 + l];
        }
    }

    __syncthreads();

    for (int stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (tid < stride) {
            for (int l = 0; l < 3; l++) {
                s_p0[tid * 3 + l] = m31_add(s_p0[tid * 3 + l], s_p0[(tid + stride) * 3 + l]);
                s_p1[tid * 3 + l] = m31_add(s_p1[tid * 3 + l], s_p1[(tid + stride) * 3 + l]);
                s_p2[tid * 3 + l] = m31_add(s_p2[tid * 3 + l], s_p2[(tid + stride) * 3 + l]);
            }
        }
        __syncthreads();
    }

    if (tid == 0) {
        int out_base = blockIdx.x * 9;
        for (int l = 0; l < 3; l++) {
            d_dst[out_base + l]     = s_p0[l];
            d_dst[out_base + 3 + l] = s_p1[l];
            d_dst[out_base + 6 + l] = s_p2[l];
        }
    }
}

// ============================================================================
// receive_challenge: bookkeeping update kernel
// ============================================================================

// For non-first round: update bk_f and bk_hg in-place
// bk_f[i]  = bk_f[2i] + (bk_f[2i+1] - bk_f[2i]) * r   (using .scale(&r))
// bk_hg[i] = bk_hg[2i] + (bk_hg[2i+1] - bk_hg[2i]) * r
//
// The Rust code uses .scale(&r) which multiplies each ext3 limb by the
// corresponding limb of r (component-wise scaling by challenge).
// For M31ext3 with ChallengeField = M31ext3, scale is full ext3 * ext3 mul.
__global__ void receive_challenge_kernel_m31ext3(
    uint32_t* __restrict__ d_bk_f,
    uint32_t* __restrict__ d_bk_hg,
    const uint32_t* __restrict__ d_r,  // 3 u32 (M31ext3)
    uint32_t eval_size
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= (int)eval_size) return;

    // Load challenge
    M31ext3 r;
    r.v[0] = d_r[0];
    r.v[1] = d_r[1];
    r.v[2] = d_r[2];

    // Update bk_f: bk_f[i] = bk_f[2i] + (bk_f[2i+1] - bk_f[2i]) * r
    {
        int base = i * 6; // 2 * i * 3
        M31ext3 f0, f1;
        f0.v[0] = d_bk_f[base + 0]; f0.v[1] = d_bk_f[base + 1]; f0.v[2] = d_bk_f[base + 2];
        f1.v[0] = d_bk_f[base + 3]; f1.v[1] = d_bk_f[base + 4]; f1.v[2] = d_bk_f[base + 5];

        M31ext3 diff = m31ext3_sub(f1, f0);
        M31ext3 scaled = m31ext3_mul(diff, r);  // .scale(&r) = full ext mul
        M31ext3 result = m31ext3_add(f0, scaled);

        int out = i * 3;
        d_bk_f[out + 0] = result.v[0];
        d_bk_f[out + 1] = result.v[1];
        d_bk_f[out + 2] = result.v[2];
    }

    // Update bk_hg: bk_hg[i] = bk_hg[2i] + (bk_hg[2i+1] - bk_hg[2i]) * r
    {
        int base = i * 6;
        M31ext3 h0, h1;
        h0.v[0] = d_bk_hg[base + 0]; h0.v[1] = d_bk_hg[base + 1]; h0.v[2] = d_bk_hg[base + 2];
        h1.v[0] = d_bk_hg[base + 3]; h1.v[1] = d_bk_hg[base + 4]; h1.v[2] = d_bk_hg[base + 5];

        M31ext3 diff = m31ext3_sub(h1, h0);
        M31ext3 scaled = m31ext3_mul(diff, r);
        M31ext3 result = m31ext3_add(h0, scaled);

        int out = i * 3;
        d_bk_hg[out + 0] = result.v[0];
        d_bk_hg[out + 1] = result.v[1];
        d_bk_hg[out + 2] = result.v[2];
    }
}

// First-round variant: read from init_v (M31 base field, 1 u32 each)
// bk_f[i] = r * (init_v[2i+1] - init_v[2i]) + init_v[2i]
// where init_v are base field M31 and r is M31ext3
__global__ void receive_challenge_first_round_kernel(
    uint32_t* __restrict__ d_bk_f,
    uint32_t* __restrict__ d_bk_hg,
    const uint32_t* __restrict__ d_r,
    const uint32_t* __restrict__ d_init_v,  // M31 base field (1 u32 each)
    uint32_t eval_size
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= (int)eval_size) return;

    // Load challenge (M31ext3)
    M31ext3 r;
    r.v[0] = d_r[0];
    r.v[1] = d_r[1];
    r.v[2] = d_r[2];

    // bk_f[i] = r * (init_v[2i+1] - init_v[2i]) + init_v[2i]
    // init_v values are M31 base field; we embed them into M31ext3 (limb0 = val, limb1,2 = 0)
    {
        uint32_t v0_base = d_init_v[2 * i];
        uint32_t v1_base = d_init_v[2 * i + 1];
        uint32_t diff_base = m31_sub(v1_base, v0_base);

        // r * diff_base: scale M31ext3 by M31 base field element
        M31ext3 scaled;
        scaled.v[0] = m31_mul(r.v[0], diff_base);
        scaled.v[1] = m31_mul(r.v[1], diff_base);
        scaled.v[2] = m31_mul(r.v[2], diff_base);

        // Add init_v[2i] (embedded as M31ext3 with limbs [v0_base, 0, 0])
        M31ext3 result;
        result.v[0] = m31_add(scaled.v[0], v0_base);
        result.v[1] = scaled.v[1];
        result.v[2] = scaled.v[2];

        int out = i * 3;
        d_bk_f[out + 0] = result.v[0];
        d_bk_f[out + 1] = result.v[1];
        d_bk_f[out + 2] = result.v[2];
    }

    // Update bk_hg same as non-first-round (bk_hg is already M31ext3)
    {
        int base = i * 6;
        M31ext3 h0, h1;
        h0.v[0] = d_bk_hg[base + 0]; h0.v[1] = d_bk_hg[base + 1]; h0.v[2] = d_bk_hg[base + 2];
        h1.v[0] = d_bk_hg[base + 3]; h1.v[1] = d_bk_hg[base + 4]; h1.v[2] = d_bk_hg[base + 5];

        M31ext3 diff = m31ext3_sub(h1, h0);
        M31ext3 scaled = m31ext3_mul(diff, r);
        M31ext3 result = m31ext3_add(h0, scaled);

        int out = i * 3;
        d_bk_hg[out + 0] = result.v[0];
        d_bk_hg[out + 1] = result.v[1];
        d_bk_hg[out + 2] = result.v[2];
    }
}

// ============================================================================
// Host wrapper functions with extern "C" linkage
// ============================================================================

#define SUMCHECK_BLOCK_SIZE 256

extern "C" int cuda_m31ext3_poly_eval(
    const uint32_t* d_bk_f,
    const uint32_t* d_bk_hg,
    uint32_t* d_result,
    uint32_t eval_size
) {
    if (eval_size == 0) {
        // Zero out result
        cudaError_t err = cudaMemset(d_result, 0, 9 * sizeof(uint32_t));
        return (err == cudaSuccess) ? 0 : (int)err;
    }

    int block_size = SUMCHECK_BLOCK_SIZE;
    int num_blocks = (eval_size + block_size - 1) / block_size;
    size_t smem_size = 9 * block_size * sizeof(uint32_t);  // 3 * blockDim * 3 limbs

    // Allocate block results buffer
    uint32_t* d_block_results = nullptr;
    cudaError_t err = cudaMalloc(&d_block_results, num_blocks * 9 * sizeof(uint32_t));
    if (err != cudaSuccess) return (int)err;

    // Launch first reduction
    poly_eval_kernel_m31ext3<<<num_blocks, block_size, smem_size>>>(
        d_bk_f, d_bk_hg, d_block_results, eval_size
    );

    // Iteratively reduce block results until we have 1 block
    uint32_t* d_reduce_src = d_block_results;
    uint32_t* d_reduce_dst = nullptr;
    int current_blocks = num_blocks;

    while (current_blocks > 1) {
        int next_blocks = (current_blocks + block_size - 1) / block_size;
        err = cudaMalloc(&d_reduce_dst, next_blocks * 9 * sizeof(uint32_t));
        if (err != cudaSuccess) {
            cudaFree(d_block_results);
            return (int)err;
        }

        reduce_blocks_m31ext3<<<next_blocks, block_size, smem_size>>>(
            d_reduce_src, d_reduce_dst, current_blocks
        );

        if (d_reduce_src != d_block_results) {
            cudaFree(d_reduce_src);
        }
        d_reduce_src = d_reduce_dst;
        d_reduce_dst = nullptr;
        current_blocks = next_blocks;
    }

    // Copy final result (9 u32s = [p0, p1, p2])
    err = cudaMemcpy(d_result, d_reduce_src, 9 * sizeof(uint32_t), cudaMemcpyDeviceToDevice);

    // Cleanup
    if (d_reduce_src != d_block_results) {
        cudaFree(d_reduce_src);
    }
    cudaFree(d_block_results);

    err = cudaGetLastError();
    return (err == cudaSuccess) ? 0 : (int)err;
}

extern "C" int cuda_m31ext3_receive_challenge(
    uint32_t* d_bk_f,
    uint32_t* d_bk_hg,
    const uint32_t* d_challenge_r,
    uint32_t eval_size,
    int first_round,
    const uint32_t* d_init_v
) {
    if (eval_size == 0) return 0;

    int block_size = SUMCHECK_BLOCK_SIZE;
    int num_blocks = (eval_size + block_size - 1) / block_size;

    if (first_round && d_init_v != nullptr) {
        receive_challenge_first_round_kernel<<<num_blocks, block_size>>>(
            d_bk_f, d_bk_hg, d_challenge_r, d_init_v, eval_size
        );
    } else {
        receive_challenge_kernel_m31ext3<<<num_blocks, block_size>>>(
            d_bk_f, d_bk_hg, d_challenge_r, eval_size
        );
    }

    cudaError_t err = cudaGetLastError();
    if (err != cudaSuccess) return (int)err;

    err = cudaDeviceSynchronize();
    return (err == cudaSuccess) ? 0 : (int)err;
}
