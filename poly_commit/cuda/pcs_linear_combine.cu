// pcs_linear_combine.cu — GPU kernel for PCS linear combine, callable from Rust via FFI.
//
// extern "C" void gpu_linear_combine_m31ext3(
//     const uint32_t* packed_evals, // commit_len * 16 uint32_t (M31x16)
//     const uint32_t* eq_coeffs,    // packed_rows * 16 * 3 uint32_t (M31Ext3 as 3 M31)
//     uint32_t packed_rows,
//     uint32_t msg_len,
//     uint32_t* eval_row,           // msg_len * 3 uint32_t (M31Ext3 output)
//     uint32_t n_proximity,
//     const uint32_t* rand_coeffs,  // n_proximity * packed_rows * 16 * 3 uint32_t
//     uint32_t* prox_rows           // n_proximity * msg_len * 3 uint32_t
// );

#include <cstdint>
#include <cstdio>

static constexpr uint32_t M31_P = (1u << 31) - 1;

__device__ static inline uint32_t m31_add(uint32_t a, uint32_t b) {
    uint32_t s = a + b;
    return s >= M31_P ? s - M31_P : s;
}

__device__ static inline uint32_t m31_mul(uint32_t a, uint32_t b) {
    uint64_t p = (uint64_t)a * b;
    uint32_t lo = (uint32_t)(p & M31_P);
    uint32_t hi = (uint32_t)(p >> 31);
    uint32_t s = lo + hi;
    return s >= M31_P ? s - M31_P : s;
}

// M31Ext3 * M31 scale: out[c] += coeff[c] * val for c=0,1,2
// Each thread computes one eval_row[j] = sum over packed_rows of sum over 16 lanes of
//   eq_coeffs[r*16+k] * packed_evals[(r*msg_len+j)*16+k]
// where eq_coeffs is M31Ext3 (3 components) and packed_evals is M31 (1 component)
__global__ void kernel_lc(
    const uint32_t* packed_evals,  // [commit_len * 16] M31 values
    const uint32_t* eq_coeffs,     // [packed_rows * 16 * 3] M31Ext3 as flat M31
    uint32_t packed_rows,
    uint32_t msg_len,
    uint32_t* eval_row)            // [msg_len * 3] M31Ext3 output
{
    uint32_t j = blockIdx.x * blockDim.x + threadIdx.x;
    if (j >= msg_len) return;

    uint32_t s0 = 0, s1 = 0, s2 = 0;
    for (uint32_t r = 0; r < packed_rows; r++) {
        for (uint32_t k = 0; k < 16; k++) {
            uint32_t coeff_idx = (r * 16 + k) * 3;
            uint32_t c0 = eq_coeffs[coeff_idx];
            uint32_t c1 = eq_coeffs[coeff_idx + 1];
            uint32_t c2 = eq_coeffs[coeff_idx + 2];
            uint64_t elem_idx = (uint64_t)(r * msg_len + j) * 16 + k;
            uint32_t val = packed_evals[elem_idx];
            s0 = m31_add(s0, m31_mul(c0, val));
            s1 = m31_add(s1, m31_mul(c1, val));
            s2 = m31_add(s2, m31_mul(c2, val));
        }
    }
    eval_row[j * 3] = s0;
    eval_row[j * 3 + 1] = s1;
    eval_row[j * 3 + 2] = s2;
}

// Persistent GPU state
static uint32_t* d_evals = nullptr;
static size_t d_evals_size = 0;
static uint32_t* d_result = nullptr;
static size_t d_result_size = 0;

extern "C" void gpu_linear_combine_m31ext3(
    const uint32_t* h_packed_evals,
    const uint32_t* h_eq_coeffs,
    uint32_t packed_rows,
    uint32_t msg_len,
    uint32_t* h_eval_row,
    uint32_t n_proximity,
    const uint32_t* h_rand_coeffs,
    uint32_t* h_prox_rows)
{
    uint32_t commit_len = packed_rows * msg_len;
    size_t evals_bytes = (size_t)commit_len * 16 * 4;
    size_t coeff_bytes = (size_t)packed_rows * 16 * 3 * 4;
    size_t result_bytes = (size_t)msg_len * 3 * 4;

    // Allocate/reuse device buffers
    if (evals_bytes > d_evals_size) {
        if (d_evals) cudaFree(d_evals);
        cudaMalloc(&d_evals, evals_bytes);
        d_evals_size = evals_bytes;
    }
    if (result_bytes > d_result_size) {
        if (d_result) cudaFree(d_result);
        cudaMalloc(&d_result, result_bytes);
        d_result_size = result_bytes;
    }

    // Upload packed_evals (only once per commit, reused across proximity tests)
    cudaMemcpy(d_evals, h_packed_evals, evals_bytes, cudaMemcpyHostToDevice);

    // Upload eq_coeffs and compute eval_row
    uint32_t* d_coeffs;
    cudaMalloc(&d_coeffs, coeff_bytes);
    cudaMemcpy(d_coeffs, h_eq_coeffs, coeff_bytes, cudaMemcpyHostToDevice);
    cudaMemset(d_result, 0, result_bytes);

    uint32_t nb = (msg_len + 255) / 256;
    kernel_lc<<<nb, 256>>>(d_evals, d_coeffs, packed_rows, msg_len, d_result);
    cudaMemcpy(h_eval_row, d_result, result_bytes, cudaMemcpyDeviceToHost);
    cudaFree(d_coeffs);

    // Proximity rows
    for (uint32_t t = 0; t < n_proximity; t++) {
        cudaMalloc(&d_coeffs, coeff_bytes);
        cudaMemcpy(d_coeffs, h_rand_coeffs + t * packed_rows * 16 * 3, coeff_bytes, cudaMemcpyHostToDevice);
        cudaMemset(d_result, 0, result_bytes);
        kernel_lc<<<nb, 256>>>(d_evals, d_coeffs, packed_rows, msg_len, d_result);
        cudaMemcpy(h_prox_rows + t * msg_len * 3, d_result, result_bytes, cudaMemcpyDeviceToHost);
        cudaFree(d_coeffs);
    }

    cudaDeviceSynchronize();
}
