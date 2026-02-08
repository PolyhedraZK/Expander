// CUDA kernel declarations for Orion PCS operations
// Currently: GPU-accelerated matrix transpose for commit_encoded()

#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// GPU-accelerated matrix transpose.
///
/// Transposes a rows x cols matrix of M31 elements (4 bytes each).
/// Uses tiled approach with shared memory for coalesced access.
///
/// d_input:  Device pointer to input matrix (rows * cols * 4 bytes), row-major
/// d_output: Device pointer to output matrix (cols * rows * 4 bytes), row-major
/// rows:     Number of rows in input
/// cols:     Number of columns in input
///
/// Returns 0 on success.
int cuda_m31_transpose(
    const uint32_t* d_input,
    uint32_t* d_output,
    uint32_t rows,
    uint32_t cols
);

#ifdef __cplusplus
}
#endif
