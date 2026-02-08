// Device-side Keccak-256 implementation for CUDA
//
// Implements the 24-round Keccak-f[1600] permutation and Keccak-256 hash.
// Used for Merkle tree construction in Expander's tree crate.

#pragma once

#include <stdint.h>

// Keccak-f[1600] round constants
__device__ __constant__ uint64_t KECCAK_RC[24] = {
    0x0000000000000001ULL, 0x0000000000008082ULL,
    0x800000000000808AULL, 0x8000000080008000ULL,
    0x000000000000808BULL, 0x0000000080000001ULL,
    0x8000000080008081ULL, 0x8000000000008009ULL,
    0x000000000000008AULL, 0x0000000000000088ULL,
    0x0000000080008009ULL, 0x000000008000000AULL,
    0x000000008000808BULL, 0x800000000000008BULL,
    0x8000000000008089ULL, 0x8000000000008003ULL,
    0x8000000000008002ULL, 0x8000000000000080ULL,
    0x000000000000800AULL, 0x800000008000000AULL,
    0x8000000080008081ULL, 0x8000000000008080ULL,
    0x0000000080000001ULL, 0x8000000080008008ULL,
};

// Rotation offsets for Keccak rho step
__device__ __constant__ int KECCAK_ROTC[24] = {
    1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14,
    27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44
};

// Pi step permutation indices
__device__ __constant__ int KECCAK_PILN[24] = {
    10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4,
    15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1
};

// Rotate left for 64-bit
__device__ __forceinline__ uint64_t rotl64(uint64_t x, int n) {
    return (x << n) | (x >> (64 - n));
}

// Keccak-f[1600] permutation (24 rounds)
__device__ void keccak_f1600(uint64_t state[25]) {
    uint64_t t, bc[5];

    for (int round = 0; round < 24; round++) {
        // Theta step
        for (int i = 0; i < 5; i++) {
            bc[i] = state[i] ^ state[i + 5] ^ state[i + 10] ^ state[i + 15] ^ state[i + 20];
        }
        for (int i = 0; i < 5; i++) {
            t = bc[(i + 4) % 5] ^ rotl64(bc[(i + 1) % 5], 1);
            for (int j = 0; j < 25; j += 5) {
                state[j + i] ^= t;
            }
        }

        // Rho and Pi steps
        t = state[1];
        for (int i = 0; i < 24; i++) {
            int j = KECCAK_PILN[i];
            bc[0] = state[j];
            state[j] = rotl64(t, KECCAK_ROTC[i]);
            t = bc[0];
        }

        // Chi step
        for (int j = 0; j < 25; j += 5) {
            for (int i = 0; i < 5; i++) {
                bc[i] = state[j + i];
            }
            for (int i = 0; i < 5; i++) {
                state[j + i] ^= (~bc[(i + 1) % 5]) & bc[(i + 2) % 5];
            }
        }

        // Iota step
        state[0] ^= KECCAK_RC[round];
    }
}

// Keccak-256: hash arbitrary input to 32 bytes
// For Merkle tree, we only need two cases:
//   1. Leaf hash: 64 bytes -> 32 bytes (LEAF_BYTES input)
//   2. Node hash: 64 bytes -> 32 bytes (two 32-byte children)
// Both fit in a single Keccak block (rate = 136 bytes for Keccak-256)
__device__ void keccak256_64bytes(const uint8_t input[64], uint8_t output[32]) {
    uint64_t state[25];
    for (int i = 0; i < 25; i++) state[i] = 0;

    // Absorb 64 bytes (8 uint64_t lanes)
    for (int i = 0; i < 8; i++) {
        uint64_t lane = 0;
        for (int b = 0; b < 8; b++) {
            lane |= ((uint64_t)input[i * 8 + b]) << (b * 8);
        }
        state[i] ^= lane;
    }

    // Padding: Keccak uses multi-rate padding 0x01 ... 0x80
    // For rate=136 (17 lanes), first padding byte at position 64
    state[8] ^= 0x01ULL;             // byte 64 = 0x01
    state[16] ^= 0x8000000000000000ULL; // last byte of rate block = 0x80

    keccak_f1600(state);

    // Squeeze 32 bytes (4 uint64_t lanes)
    for (int i = 0; i < 4; i++) {
        for (int b = 0; b < 8; b++) {
            output[i * 8 + b] = (uint8_t)(state[i] >> (b * 8));
        }
    }
}
