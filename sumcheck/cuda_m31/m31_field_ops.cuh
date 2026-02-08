// M31 (Mersenne-31) field operations for CUDA
// Extracted and adapted from Expander's sumcheck/cuda/include/field/M31.cuh
// These are device-only functions used by the sumcheck GPU kernels.

#pragma once

#include <stdint.h>

#define M31_MOD 2147483647u  // 2^31 - 1

// Reduce a uint64_t to M31 range
__device__ __forceinline__ uint32_t m31_reduce64(uint64_t x) {
    uint32_t lo = (uint32_t)(x & M31_MOD);
    uint32_t hi = (uint32_t)(x >> 31);
    uint32_t r = lo + hi;
    return r >= M31_MOD ? r - M31_MOD : r;
}

// M31 addition
__device__ __forceinline__ uint32_t m31_add(uint32_t a, uint32_t b) {
    uint32_t r = a + b;
    return r >= M31_MOD ? r - M31_MOD : r;
}

// M31 subtraction
__device__ __forceinline__ uint32_t m31_sub(uint32_t a, uint32_t b) {
    // If a < b, we need to add MOD to avoid underflow
    return a >= b ? a - b : a + M31_MOD - b;
}

// M31 negation
__device__ __forceinline__ uint32_t m31_neg(uint32_t a) {
    return a == 0 ? 0 : M31_MOD - a;
}

// M31 multiplication
__device__ __forceinline__ uint32_t m31_mul(uint32_t a, uint32_t b) {
    uint64_t prod = (uint64_t)a * (uint64_t)b;
    return m31_reduce64(prod);
}

// M31ext3 (cubic extension of M31) element: 3 limbs
// Extension polynomial: x^3 - 5 (i.e., the irreducible polynomial is x^3 = 5)
struct M31ext3 {
    uint32_t v[3];
};

// M31ext3 addition
__device__ __forceinline__ M31ext3 m31ext3_add(M31ext3 a, M31ext3 b) {
    M31ext3 r;
    r.v[0] = m31_add(a.v[0], b.v[0]);
    r.v[1] = m31_add(a.v[1], b.v[1]);
    r.v[2] = m31_add(a.v[2], b.v[2]);
    return r;
}

// M31ext3 subtraction
__device__ __forceinline__ M31ext3 m31ext3_sub(M31ext3 a, M31ext3 b) {
    M31ext3 r;
    r.v[0] = m31_sub(a.v[0], b.v[0]);
    r.v[1] = m31_sub(a.v[1], b.v[1]);
    r.v[2] = m31_sub(a.v[2], b.v[2]);
    return r;
}

// M31ext3 multiplication
// res[0] = a[0]*b[0] + 5*(a[1]*b[2] + a[2]*b[1])
// res[1] = a[0]*b[1] + a[1]*b[0] + 5*a[2]*b[2]
// res[2] = a[0]*b[2] + a[1]*b[1] + a[2]*b[0]
__device__ __forceinline__ M31ext3 m31ext3_mul(M31ext3 a, M31ext3 b) {
    M31ext3 r;
    r.v[0] = m31_add(
        m31_mul(a.v[0], b.v[0]),
        m31_mul(5, m31_add(m31_mul(a.v[1], b.v[2]), m31_mul(a.v[2], b.v[1])))
    );
    r.v[1] = m31_add(
        m31_add(m31_mul(a.v[0], b.v[1]), m31_mul(a.v[1], b.v[0])),
        m31_mul(5, m31_mul(a.v[2], b.v[2]))
    );
    r.v[2] = m31_add(
        m31_add(m31_mul(a.v[0], b.v[2]), m31_mul(a.v[1], b.v[1])),
        m31_mul(a.v[2], b.v[0])
    );
    return r;
}

// M31ext3 zero
__device__ __forceinline__ M31ext3 m31ext3_zero() {
    M31ext3 r;
    r.v[0] = 0; r.v[1] = 0; r.v[2] = 0;
    return r;
}

// Scale M31ext3 by M31 scalar
__device__ __forceinline__ M31ext3 m31ext3_scale(M31ext3 a, uint32_t s) {
    M31ext3 r;
    r.v[0] = m31_mul(a.v[0], s);
    r.v[1] = m31_mul(a.v[1], s);
    r.v[2] = m31_mul(a.v[2], s);
    return r;
}

// Multiply M31ext3 by M31 base field element (embed base into ext)
__device__ __forceinline__ M31ext3 m31ext3_mul_base(M31ext3 a, uint32_t b) {
    return m31ext3_scale(a, b);
}
