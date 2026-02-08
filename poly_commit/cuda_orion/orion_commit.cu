// CUDA kernels for Orion PCS commit acceleration
//
// Implements GPU-accelerated matrix transpose using tiled shared memory
// for coalesced global memory access. This is the memory-bandwidth-bound
// operation in commit_encoded() that benefits from GPU's high memory bandwidth.

#include "orion_commit.cuh"
#include <stdio.h>

// Tile size for shared memory transpose.
// 32x32 tiles are optimal for coalescing on modern GPUs.
// We add 1 to the column dimension to avoid bank conflicts.
#define TILE_DIM 32
#define BLOCK_ROWS 8  // Each thread block processes TILE_DIM * BLOCK_ROWS elements

// ============================================================================
// Tiled matrix transpose kernel
// ============================================================================

// Each thread block transposes one TILE_DIM x TILE_DIM tile.
// Threads load from input with coalesced reads, store to shared memory,
// then write to output with coalesced writes (transposed).
__global__ void transpose_kernel(
    const uint32_t* __restrict__ d_input,
    uint32_t* __restrict__ d_output,
    uint32_t rows,
    uint32_t cols
) {
    // Shared memory tile with padding to avoid bank conflicts
    __shared__ uint32_t tile[TILE_DIM][TILE_DIM + 1];

    // Input indices
    int x = blockIdx.x * TILE_DIM + threadIdx.x;
    int y = blockIdx.y * TILE_DIM + threadIdx.y;

    // Load tile from input (coalesced reads along rows)
    for (int j = 0; j < TILE_DIM; j += BLOCK_ROWS) {
        if (x < (int)cols && (y + j) < (int)rows) {
            tile[threadIdx.y + j][threadIdx.x] = d_input[(y + j) * cols + x];
        }
    }

    __syncthreads();

    // Output indices (transposed)
    x = blockIdx.y * TILE_DIM + threadIdx.x;
    y = blockIdx.x * TILE_DIM + threadIdx.y;

    // Write tile to output (coalesced writes along transposed rows)
    for (int j = 0; j < TILE_DIM; j += BLOCK_ROWS) {
        if (x < (int)rows && (y + j) < (int)cols) {
            d_output[(y + j) * rows + x] = tile[threadIdx.x][threadIdx.y + j];
        }
    }
}

// ============================================================================
// Host wrapper
// ============================================================================

extern "C" int cuda_m31_transpose(
    const uint32_t* d_input,
    uint32_t* d_output,
    uint32_t rows,
    uint32_t cols
) {
    if (rows == 0 || cols == 0) return 0;

    dim3 block(TILE_DIM, BLOCK_ROWS);
    dim3 grid(
        (cols + TILE_DIM - 1) / TILE_DIM,
        (rows + TILE_DIM - 1) / TILE_DIM
    );

    transpose_kernel<<<grid, block>>>(d_input, d_output, rows, cols);

    cudaError_t err = cudaGetLastError();
    if (err != cudaSuccess) return (int)err;

    err = cudaDeviceSynchronize();
    return (err == cudaSuccess) ? 0 : (int)err;
}
