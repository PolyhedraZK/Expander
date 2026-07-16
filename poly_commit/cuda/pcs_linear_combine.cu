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

// GPU eval at challenge: result = sum_i eq(challenge, i) * packed_vals[i]
// Each thread handles multiple elements, block reduces to partial sum.
__global__ void kernel_eval_dot(
    const uint32_t* packed_vals,  // [commit_len * 16] M31
    const uint32_t* eq_rz,       // [commit_len/16 * 3] M31Ext3 (eq over circuit vars)
    const uint32_t* eq_simd,     // [16 * 3] M31Ext3 (eq over SIMD vars)
    uint32_t n_elements,          // commit_len / 16 (number of M31x16 groups... wait)
    uint32_t* partial_sums)       // [gridDim.x * 3] M31Ext3 partial sums
{
    // Actually: commit_len M31x16 elements. eq_rz has commit_len entries.
    // eq_simd has 16 entries.
    // result = sum over p of sum over k of eq_rz[p] * eq_simd[k] * vals[p*16+k]
    // But eq_rz and eq_simd are M31Ext3 (3 components), vals are M31.
    // eq_rz[p] * eq_simd[k] is E3*E3 = E3 (9 muls + 6 adds).
    // Then * vals = E3.scale(M31) (3 muls).

    extern __shared__ uint32_t smem[];
    uint32_t tid = threadIdx.x, bs = blockDim.x;
    uint32_t s0 = 0, s1 = 0, s2 = 0;

    for (uint32_t p = blockIdx.x * bs + tid; p < n_elements; p += gridDim.x * bs) {
        // eq_rz[p] as E3
        uint32_t rz0 = eq_rz[p*3], rz1 = eq_rz[p*3+1], rz2 = eq_rz[p*3+2];
        for (uint32_t k = 0; k < 16; k++) {
            // eq_simd[k] as E3
            uint32_t sk0 = eq_simd[k*3], sk1 = eq_simd[k*3+1], sk2 = eq_simd[k*3+2];
            // combined = rz * simd (E3 * E3)
            // c0 = rz0*sk0 + 5*(rz1*sk2 + rz2*sk1)
            // c1 = rz0*sk1 + rz1*sk0 + 5*(rz2*sk2)
            // c2 = rz0*sk2 + rz1*sk1 + rz2*sk0
            uint32_t c0 = m31_add(m31_mul(rz0,sk0), m31_mul(5, m31_add(m31_mul(rz1,sk2), m31_mul(rz2,sk1))));
            uint32_t c1 = m31_add(m31_add(m31_mul(rz0,sk1), m31_mul(rz1,sk0)), m31_mul(5, m31_mul(rz2,sk2)));
            uint32_t c2 = m31_add(m31_add(m31_mul(rz0,sk2), m31_mul(rz1,sk1)), m31_mul(rz2,sk0));
            // scale by val
            uint32_t val = packed_vals[(uint64_t)p * 16 + k];
            s0 = m31_add(s0, m31_mul(c0, val));
            s1 = m31_add(s1, m31_mul(c1, val));
            s2 = m31_add(s2, m31_mul(c2, val));
        }
    }

    // Block reduction
    smem[tid*3] = s0; smem[tid*3+1] = s1; smem[tid*3+2] = s2;
    __syncthreads();
    for (uint32_t s = bs/2; s > 0; s >>= 1) {
        if (tid < s) {
            smem[tid*3] = m31_add(smem[tid*3], smem[(tid+s)*3]);
            smem[tid*3+1] = m31_add(smem[tid*3+1], smem[(tid+s)*3+1]);
            smem[tid*3+2] = m31_add(smem[tid*3+2], smem[(tid+s)*3+2]);
        }
        __syncthreads();
    }
    if (tid == 0) {
        partial_sums[blockIdx.x*3] = smem[0];
        partial_sums[blockIdx.x*3+1] = smem[1];
        partial_sums[blockIdx.x*3+2] = smem[2];
    }
}

// Final reduction of partial sums
__global__ void kernel_eval_reduce(const uint32_t* partials, uint32_t n_blocks, uint32_t* result) {
    uint32_t s0 = 0, s1 = 0, s2 = 0;
    for (uint32_t i = threadIdx.x; i < n_blocks; i += blockDim.x) {
        s0 = m31_add(s0, partials[i*3]);
        s1 = m31_add(s1, partials[i*3+1]);
        s2 = m31_add(s2, partials[i*3+2]);
    }
    extern __shared__ uint32_t smem[];
    smem[threadIdx.x*3] = s0; smem[threadIdx.x*3+1] = s1; smem[threadIdx.x*3+2] = s2;
    __syncthreads();
    for (uint32_t s = blockDim.x/2; s > 0; s >>= 1) {
        if (threadIdx.x < s) {
            smem[threadIdx.x*3] = m31_add(smem[threadIdx.x*3], smem[(threadIdx.x+s)*3]);
            smem[threadIdx.x*3+1] = m31_add(smem[threadIdx.x*3+1], smem[(threadIdx.x+s)*3+1]);
            smem[threadIdx.x*3+2] = m31_add(smem[threadIdx.x*3+2], smem[(threadIdx.x+s)*3+2]);
        }
        __syncthreads();
    }
    if (threadIdx.x == 0) { result[0] = smem[0]; result[1] = smem[1]; result[2] = smem[2]; }
}

// Build eq table: eq[b] = prod_i (b_i ? r_i : (1-r_i))
__global__ void kernel_build_eq_ffi(const uint32_t* challenges, int n_vars, uint32_t* eq_table) {
    uint32_t b = blockIdx.x * blockDim.x + threadIdx.x;
    if (b >= (1u << n_vars)) return;
    // Each challenge is M31Ext3 = 3 uint32_t
    // eq value is also M31Ext3
    // Start with E3::one() = (1, 0, 0)
    uint32_t v0 = 1, v1 = 0, v2 = 0;
    for (int i = 0; i < n_vars; i++) {
        uint32_t r0 = challenges[i*3], r1 = challenges[i*3+1], r2 = challenges[i*3+2];
        // factor = (b>>i)&1 ? (r0,r1,r2) : (1-r0, -r1, -r2) = (M31_P-r0+1, M31_P-r1, M31_P-r2)
        uint32_t f0, f1, f2;
        if ((b >> i) & 1) { f0 = r0; f1 = r1; f2 = r2; }
        else {
            f0 = (M31_P + 1 - r0); if (f0 >= M31_P) f0 -= M31_P;
            f1 = M31_P - r1; f2 = M31_P - r2;
        }
        // val = val * factor (E3 * E3)
        uint32_t n0 = m31_add(m31_mul(v0,f0), m31_mul(5, m31_add(m31_mul(v1,f2), m31_mul(v2,f1))));
        uint32_t n1 = m31_add(m31_add(m31_mul(v0,f1), m31_mul(v1,f0)), m31_mul(5, m31_mul(v2,f2)));
        uint32_t n2 = m31_add(m31_add(m31_mul(v0,f2), m31_mul(v1,f1)), m31_mul(v2,f0));
        v0 = n0; v1 = n1; v2 = n2;
    }
    eq_table[b*3] = v0; eq_table[b*3+1] = v1; eq_table[b*3+2] = v2;
}

// Forward declarations
static uint32_t* d_evals = nullptr;
static size_t d_evals_size = 0;
extern "C" const uint32_t* gpu_tree_find_poly(const uint32_t* h_ptr, uint32_t len);

static void ensure_buf(uint32_t** p, size_t* cap, size_t needed) {
    if (*cap >= needed) return;
    if (*p) cudaFree(*p);
    cudaMalloc(p, needed); *cap = needed;
}

// ================================================================
// Blake3 for Merkle tree (matching Rust blake3 crate for 64-byte input)
// ================================================================
__host__ __device__ static inline uint32_t rotr32(uint32_t x, int n) { return (x >> n) | (x << (32 - n)); }
__host__ __device__ static inline void b3_g(uint32_t* s, int a, int b, int c, int d, uint32_t mx, uint32_t my) {
    s[a]=s[a]+s[b]+mx; s[d]=rotr32(s[d]^s[a],16); s[c]=s[c]+s[d]; s[b]=rotr32(s[b]^s[c],12);
    s[a]=s[a]+s[b]+my; s[d]=rotr32(s[d]^s[a],8); s[c]=s[c]+s[d]; s[b]=rotr32(s[b]^s[c],7);
}
__host__ __device__ static inline void b3_round(uint32_t* s, const uint32_t* m) {
    b3_g(s,0,4,8,12,m[0],m[1]); b3_g(s,1,5,9,13,m[2],m[3]);
    b3_g(s,2,6,10,14,m[4],m[5]); b3_g(s,3,7,11,15,m[6],m[7]);
    b3_g(s,0,5,10,15,m[8],m[9]); b3_g(s,1,6,11,12,m[10],m[11]);
    b3_g(s,2,7,8,13,m[12],m[13]); b3_g(s,3,4,9,14,m[14],m[15]);
}
__host__ __device__ static inline void b3_permute(uint32_t* m) {
    uint32_t t[16]={m[2],m[6],m[3],m[10],m[7],m[0],m[4],m[13],m[1],m[11],m[12],m[5],m[9],m[14],m[15],m[8]};
    for(int i=0;i<16;i++)m[i]=t[i];
}
__device__ static void b3_hash64(const uint8_t* data, uint8_t* out) {
    uint32_t iv[8]={0x6A09E667,0xBB67AE85,0x3C6EF372,0xA54FF53A,0x510E527F,0x9B05688C,0x1F83D9AB,0x5BE0CD19};
    uint32_t blk[16]; for(int i=0;i<16;i++) blk[i]=(uint32_t)data[4*i]|((uint32_t)data[4*i+1]<<8)|((uint32_t)data[4*i+2]<<16)|((uint32_t)data[4*i+3]<<24);
    uint32_t s[16]={iv[0],iv[1],iv[2],iv[3],iv[4],iv[5],iv[6],iv[7],iv[0],iv[1],iv[2],iv[3],0,0,64,0x0B};
    uint32_t m[16]; for(int i=0;i<16;i++)m[i]=blk[i];
    b3_round(s,m);b3_permute(m);b3_round(s,m);b3_permute(m);b3_round(s,m);b3_permute(m);
    b3_round(s,m);b3_permute(m);b3_round(s,m);b3_permute(m);b3_round(s,m);b3_permute(m);b3_round(s,m);
    for(int i=0;i<8;i++){uint32_t v=s[i]^s[i+8]; out[4*i]=(uint8_t)v;out[4*i+1]=(uint8_t)(v>>8);out[4*i+2]=(uint8_t)(v>>16);out[4*i+3]=(uint8_t)(v>>24);}
}

__global__ void k_hash_leaves(const uint8_t* data, uint8_t* hashes, uint32_t n) {
    uint32_t i = blockIdx.x*blockDim.x+threadIdx.x;
    if(i>=n)return;
    b3_hash64(data+(uint64_t)i*64, hashes+(uint64_t)i*32);
}
__global__ void k_hash_parents(const uint8_t* children, uint8_t* parents, uint32_t n) {
    uint32_t i = blockIdx.x*blockDim.x+threadIdx.x;
    if(i>=n)return;
    uint8_t buf[64];
    for(int b=0;b<32;b++){buf[b]=children[(uint64_t)i*64+b];buf[32+b]=children[(uint64_t)i*64+32+b];}
    b3_hash64(buf, parents+(uint64_t)i*32);
}

// FFI: build Merkle tree on GPU.
// Input: h_leaves (n_leaves × 64 bytes, host memory)
// Output: h_leaf_hashes (n_leaves × 32 bytes), h_nodes ((n_leaves-1) × 32 bytes)
// h_nodes[0] = root. Layout matches Expander Tree.
extern "C" void gpu_merkle_tree_blake3(
    const uint8_t* h_leaves, uint32_t n_leaves,
    uint8_t* h_leaf_hashes, uint8_t* h_nodes)
{
    uint8_t* d_leaves; cudaMalloc(&d_leaves, (size_t)n_leaves*64);
    cudaMemcpy(d_leaves, h_leaves, (size_t)n_leaves*64, cudaMemcpyHostToDevice);

    uint8_t* d_lh; cudaMalloc(&d_lh, (size_t)n_leaves*32);
    k_hash_leaves<<<(n_leaves+255)/256,256>>>(d_leaves, d_lh, n_leaves);
    cudaFree(d_leaves);

    // Build tree: nodes[(n_leaves-1)] bottom-up
    uint8_t* d_nodes; cudaMalloc(&d_nodes, (size_t)(n_leaves-1)*32);
    // Bottom level
    uint32_t level_size = n_leaves/2;
    uint32_t start = level_size-1;
    k_hash_parents<<<(level_size+255)/256,256>>>(d_lh, d_nodes+(uint64_t)start*32, level_size);
    // Higher levels
    while(level_size>1) {
        level_size/=2;
        uint32_t ps=level_size-1, cs=2*level_size-1;
        k_hash_parents<<<(level_size+255)/256,256>>>(d_nodes+(uint64_t)cs*32, d_nodes+(uint64_t)ps*32, level_size);
    }
    cudaDeviceSynchronize();

    cudaMemcpy(h_leaf_hashes, d_lh, (size_t)n_leaves*32, cudaMemcpyDeviceToHost);
    cudaMemcpy(h_nodes, d_nodes, (size_t)(n_leaves-1)*32, cudaMemcpyDeviceToHost);
    cudaFree(d_lh); cudaFree(d_nodes);
}

// Pre-allocated eval-at-challenge buffers
static uint32_t* d_eq_rz_buf = nullptr; static size_t d_eq_rz_size = 0;
static uint32_t* d_eq_simd_buf = nullptr;
static uint32_t* d_ch_buf_eval = nullptr;
static uint32_t* d_partials_buf = nullptr; static size_t d_partials_size = 0;
static uint32_t* d_result_eval = nullptr;
static bool eval_init = false;

extern "C" void gpu_eval_at_challenge_m31ext3(
    const uint32_t* h_packed_vals, uint32_t commit_len,
    const uint32_t* h_rz, uint32_t n_rz_vars,
    const uint32_t* h_r_simd,
    uint32_t* h_result)
{
    if (!eval_init) {
        cudaMalloc(&d_eq_simd_buf, 16 * 3 * 4);
        cudaMalloc(&d_ch_buf_eval, 64 * 3 * 4); // max 64 vars
        cudaMalloc(&d_result_eval, 3 * 4);
        eval_init = true;
    }

    uint32_t eq_rz_size = 1u << n_rz_vars;
    size_t eq_rz_bytes = (size_t)eq_rz_size * 3 * 4;
    if (eq_rz_bytes > d_eq_rz_size) {
        if (d_eq_rz_buf) cudaFree(d_eq_rz_buf);
        cudaMalloc(&d_eq_rz_buf, eq_rz_bytes); d_eq_rz_size = eq_rz_bytes;
    }

    cudaMemcpy(d_ch_buf_eval, h_rz, n_rz_vars * 3 * 4, cudaMemcpyHostToDevice);
    kernel_build_eq_ffi<<<(eq_rz_size+255)/256, 256>>>(d_ch_buf_eval, n_rz_vars, d_eq_rz_buf);

    cudaMemcpy(d_ch_buf_eval, h_r_simd, 4 * 3 * 4, cudaMemcpyHostToDevice);
    kernel_build_eq_ffi<<<1, 256>>>(d_ch_buf_eval, 4, d_eq_simd_buf);

    size_t evals_bytes = (size_t)commit_len * 16 * 4;
    const uint32_t* d_src = gpu_tree_find_poly(h_packed_vals, commit_len);
    if (!d_src) {
        ensure_buf(&d_evals, &d_evals_size, evals_bytes);
        cudaMemcpy(d_evals, h_packed_vals, evals_bytes, cudaMemcpyHostToDevice);
        d_src = d_evals;
    }

    uint32_t nb = 512;
    size_t part_bytes = (size_t)nb * 3 * 4;
    if (part_bytes > d_partials_size) {
        if (d_partials_buf) cudaFree(d_partials_buf);
        cudaMalloc(&d_partials_buf, part_bytes); d_partials_size = part_bytes;
    }
    kernel_eval_dot<<<nb, 128, 128*3*4>>>(d_src, d_eq_rz_buf, d_eq_simd_buf, commit_len, d_partials_buf);
    kernel_eval_reduce<<<1, 256, 256*3*4>>>(d_partials_buf, nb, d_result_eval);
    cudaMemcpy(h_result, d_result_eval, 3 * 4, cudaMemcpyDeviceToHost);
}


// Persistent GPU state — pre-allocated, never freed during proving
static uint32_t* d_result = nullptr;
static size_t d_result_size = 0;
static uint32_t* d_coeffs_buf = nullptr;
static size_t d_coeffs_size = 0;

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

    ensure_buf(&d_evals, &d_evals_size, evals_bytes);
    ensure_buf(&d_result, &d_result_size, result_bytes);
    ensure_buf(&d_coeffs_buf, &d_coeffs_size, coeff_bytes);

    cudaMemcpy(d_evals, h_packed_evals, evals_bytes, cudaMemcpyHostToDevice);

    // eval_row
    cudaMemcpy(d_coeffs_buf, h_eq_coeffs, coeff_bytes, cudaMemcpyHostToDevice);
    uint32_t nb = (msg_len + 255) / 256;
    cudaMemset(d_result, 0, result_bytes);
    kernel_lc<<<nb, 256>>>(d_evals, d_coeffs_buf, packed_rows, msg_len, d_result);
    cudaMemcpy(h_eval_row, d_result, result_bytes, cudaMemcpyDeviceToHost);

    // Proximity rows — reuse same coeff buffer
    for (uint32_t t = 0; t < n_proximity; t++) {
        cudaMemcpy(d_coeffs_buf, h_rand_coeffs + (uint64_t)t * packed_rows * 16 * 3, coeff_bytes, cudaMemcpyHostToDevice);
        cudaMemset(d_result, 0, result_bytes);
        kernel_lc<<<nb, 256>>>(d_evals, d_coeffs_buf, packed_rows, msg_len, d_result);
        cudaMemcpy(h_prox_rows + (uint64_t)t * msg_len * 3, d_result, result_bytes, cudaMemcpyDeviceToHost);
    }
}

// ================================================================
// GPU PCS Open — polynomial and tree ALREADY on GPU.
// Only small data (coefficients, query indices) uploaded. Results downloaded.
// ================================================================
// Merkle leaf extraction kernel: each thread copies one query's leaf range
__global__ void kernel_extract_leaves(
    const uint8_t* tree_leaves, const uint32_t* query_positions,
    uint32_t leaves_per_query, uint8_t* out, uint32_t n_queries)
{
    uint32_t qi = blockIdx.x * blockDim.x + threadIdx.x;
    if (qi >= n_queries) return;
    uint32_t left = query_positions[qi] * leaves_per_query;
    uint32_t bytes = leaves_per_query * 64;
    const uint8_t* src = tree_leaves + (uint64_t)left * 64;
    uint8_t* dst = out + (uint64_t)qi * bytes;
    for (uint32_t i = 0; i < bytes; i++) dst[i] = src[i];
}

// Merkle sibling extraction kernel: each thread handles one (query, depth_level) pair
__global__ void kernel_extract_siblings(
    const uint8_t* tree_nd, const uint32_t* query_positions,
    uint32_t n_leaves, uint32_t leaves_per_query, uint32_t depth,
    uint8_t* out, uint32_t n_queries)
{
    uint32_t idx = blockIdx.x * blockDim.x + threadIdx.x;
    uint32_t qi = idx / depth, d = idx % depth;
    if (qi >= n_queries) return;

    uint32_t range_idx = query_positions[qi];
    uint32_t level_n = n_leaves / leaves_per_query;
    uint32_t level_start = level_n - 1;
    for (uint32_t i = 0; i < d; i++) {
        range_idx >>= 1; level_n >>= 1; level_start = level_n - 1;
    }
    uint32_t sibling = range_idx ^ 1;
    uint32_t node_pos = level_start + sibling;
    const uint8_t* src = tree_nd + (uint64_t)node_pos * 32;
    uint8_t* dst = out + ((uint64_t)qi * depth + d) * 32;
    for (uint32_t i = 0; i < 32; i++) dst[i] = src[i];
}

// From gpu_commit.cu
extern "C" void gpu_tree_get_ptrs(int32_t id, uint32_t** leaves, uint8_t** lh, uint8_t** nd, uint32_t* n,
    uint32_t** poly, uint32_t* cl, uint32_t* ml);

// Persistent Merkle query buffers — pre-allocated, reused
static uint32_t* d_qi_buf = nullptr;
static size_t d_qi_size = 0;
static uint8_t* d_leaf_out_buf = nullptr;
static size_t d_leaf_out_size = 0;
static uint8_t* d_path_out_buf = nullptr;
static size_t d_path_out_size = 0;

extern "C" void gpu_pcs_open_with_device_data(
    const uint32_t* d_packed_evals,
    const uint32_t* h_eq_coeffs,
    uint32_t packed_rows,
    uint32_t msg_len,
    uint32_t* h_eval_row,
    uint32_t n_proximity,
    const uint32_t* h_rand_coeffs,
    uint32_t* h_prox_rows,
    int32_t tree_id,
    const uint32_t* h_query_indices,
    uint32_t n_queries,
    uint32_t leaves_per_query,
    uint8_t* h_query_leaves,
    uint8_t* h_query_path_nodes,
    uint32_t* out_tree_depth)
{
    size_t coeff_bytes = (size_t)packed_rows * 16 * 3 * 4;
    size_t result_bytes = (size_t)msg_len * 3 * 4;

    ensure_buf(&d_result, &d_result_size, result_bytes);
    ensure_buf(&d_coeffs_buf, &d_coeffs_size, coeff_bytes);

    cudaEvent_t ev_start, ev_lc, ev_prox, ev_merkle, ev_end;
    cudaEventCreate(&ev_start); cudaEventCreate(&ev_lc); cudaEventCreate(&ev_prox);
    cudaEventCreate(&ev_merkle); cudaEventCreate(&ev_end);
    cudaEventRecord(ev_start);

    // eval_row (polynomial already on GPU)
    cudaMemcpy(d_coeffs_buf, h_eq_coeffs, coeff_bytes, cudaMemcpyHostToDevice);
    uint32_t nb = (msg_len + 255) / 256;
    cudaMemset(d_result, 0, result_bytes);
    kernel_lc<<<nb, 256>>>(d_packed_evals, d_coeffs_buf, packed_rows, msg_len, d_result);
    cudaEventRecord(ev_lc);

    cudaMemcpy(h_eval_row, d_result, result_bytes, cudaMemcpyDeviceToHost);

    // Proximity rows — reuse coeff buffer
    for (uint32_t t = 0; t < n_proximity; t++) {
        cudaMemcpy(d_coeffs_buf, h_rand_coeffs + (uint64_t)t * packed_rows * 16 * 3, coeff_bytes, cudaMemcpyHostToDevice);
        cudaMemset(d_result, 0, result_bytes);
        kernel_lc<<<nb, 256>>>(d_packed_evals, d_coeffs_buf, packed_rows, msg_len, d_result);
        cudaMemcpy(h_prox_rows + (uint64_t)t * msg_len * 3, d_result, result_bytes, cudaMemcpyDeviceToHost);
    }
    cudaEventRecord(ev_prox);
    // Merkle extraction — pre-allocated buffers, no per-call malloc
    if (tree_id >= 0) {
        uint32_t* t_leaves; uint8_t* t_lh; uint8_t* t_nd; uint32_t n_leaves;
        uint32_t* t_poly; uint32_t t_cl, t_ml;
        gpu_tree_get_ptrs(tree_id, &t_leaves, &t_lh, &t_nd, &n_leaves, &t_poly, &t_cl, &t_ml);
        uint32_t depth = 0;
        { uint32_t x = n_leaves / leaves_per_query; while (x > 1) { depth++; x >>= 1; } }
        *out_tree_depth = depth;

        size_t qi_bytes = n_queries * 4;
        size_t leaf_out_bytes = (size_t)n_queries * leaves_per_query * 64;
        size_t path_out_bytes = (size_t)n_queries * depth * 32;

        ensure_buf((uint32_t**)&d_qi_buf, &d_qi_size, qi_bytes);
        if (leaf_out_bytes > d_leaf_out_size) {
            if (d_leaf_out_buf) cudaFree(d_leaf_out_buf);
            cudaMalloc(&d_leaf_out_buf, leaf_out_bytes); d_leaf_out_size = leaf_out_bytes;
        }
        if (path_out_bytes > d_path_out_size) {
            if (d_path_out_buf) cudaFree(d_path_out_buf);
            cudaMalloc(&d_path_out_buf, path_out_bytes); d_path_out_size = path_out_bytes;
        }

        cudaEventRecord(ev_merkle);
        cudaMemcpy(d_qi_buf, h_query_indices, qi_bytes, cudaMemcpyHostToDevice);
        kernel_extract_leaves<<<(n_queries+255)/256, 256>>>(
            (uint8_t*)t_leaves, d_qi_buf, leaves_per_query, d_leaf_out_buf, n_queries);
        uint32_t total_pairs = n_queries * depth;
        kernel_extract_siblings<<<(total_pairs+255)/256, 256>>>(
            t_nd, d_qi_buf, n_leaves, leaves_per_query, depth, d_path_out_buf, n_queries);
        cudaMemcpy(h_query_leaves, d_leaf_out_buf, leaf_out_bytes, cudaMemcpyDeviceToHost);
        cudaMemcpy(h_query_path_nodes, d_path_out_buf, path_out_bytes, cudaMemcpyDeviceToHost);
        cudaEventRecord(ev_end);
        cudaEventSynchronize(ev_end);
        float t_lc, t_prox, t_merkle, t_total;
        cudaEventElapsedTime(&t_lc, ev_start, ev_lc);
        cudaEventElapsedTime(&t_prox, ev_lc, ev_prox);
        cudaEventElapsedTime(&t_merkle, ev_merkle, ev_end);
        cudaEventElapsedTime(&t_total, ev_start, ev_end);
        fprintf(stderr, "      [pcs-gpu] rows=%u msg=%u nprox=%u nq=%u: lc=%.1fms prox=%.1fms merkle=%.1fms total=%.1fms\n",
            packed_rows, msg_len, n_proximity, n_queries, t_lc, t_prox, t_merkle, t_total);
    }
    cudaEventDestroy(ev_start); cudaEventDestroy(ev_lc); cudaEventDestroy(ev_prox);
    cudaEventDestroy(ev_merkle); cudaEventDestroy(ev_end);
}
