// gpu_commit.cu — GPU PCS commit: Spielman encode + transpose + Blake3 Merkle
// Uses batched M31x16 kernels (all rows in single launch) + persistent context.

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <vector>
#include <fstream>
#include <string>

// Import batched Spielman kernels and Blake3 from workspace root cuda/
#include "spielman.cuh"
#include "blake3.cuh"

// Blake3 leaf hash: 64 bytes input → 32 bytes output
__global__ void k_hash_leaf(const uint8_t*data,uint8_t*hashes,uint32_t n){
    uint32_t i=blockIdx.x*blockDim.x+threadIdx.x;if(i>=n)return;
    blake3_hash_64(data+(uint64_t)i*64,hashes+(uint64_t)i*32);
}
// Blake3 pair hash: concatenate two 32-byte children → 32-byte parent
__global__ void k_hash_pair(const uint8_t*children,uint8_t*parents,uint32_t n){
    uint32_t i=blockIdx.x*blockDim.x+threadIdx.x;if(i>=n)return;
    uint8_t buf[64];
    for(int b=0;b<32;b++){buf[b]=children[(uint64_t)i*64+b];buf[32+b]=children[(uint64_t)i*64+32+b];}
    blake3_hash_64(buf,parents+(uint64_t)i*32);
}

// ---- Persistent context ----
static struct {
    std::string cached_srs_path;
    std::vector<ExpanderGraphGpu> graphs;
    uint32_t n_graphs;

    uint32_t *d_buffer, *d_scratch, *d_transposed;
    size_t buf_cap, scratch_cap, trans_cap; // bytes

    uint8_t *d_lh, *d_nd;
    size_t lh_cap, nd_cap;

    bool initialized;
} g_ctx = {.initialized = false};

static void ensure_u32(uint32_t** p, size_t* cap, size_t needed) {
    if (*cap >= needed) return;
    if (*p) cudaFree(*p);
    cudaMalloc(p, needed);
    *cap = needed;
}
static void ensure_u8(uint8_t** p, size_t* cap, size_t needed) {
    if (*cap >= needed) return;
    if (*p) cudaFree(*p);
    cudaMalloc(p, needed);
    *cap = needed;
}

static void load_srs(const char* srs_path) {
    if (g_ctx.cached_srs_path == srs_path) return;
    for (auto& g : g_ctx.graphs) { cudaFree(g.d_row_ptrs); cudaFree(g.d_col_indices); }
    g_ctx.graphs.clear();

    std::ifstream sf(srs_path, std::ios::binary);
    uint32_t s_ml, s_cw, s_ng, s_nlpq;
    sf.read((char*)&s_ml,4); sf.read((char*)&s_cw,4); sf.read((char*)&s_ng,4); sf.read((char*)&s_nlpq,4);
    g_ctx.graphs.resize(s_ng);
    g_ctx.n_graphs = s_ng;
    for (uint32_t gi = 0; gi < s_ng; gi++) {
        auto& g = g_ctx.graphs[gi];
        sf.read((char*)&g.input_start,4); sf.read((char*)&g.output_start,4); sf.read((char*)&g.output_end,4);
        g.R = g.output_end - g.output_start + 1;
        std::vector<uint32_t> rp(g.R+1); sf.read((char*)rp.data(),(g.R+1)*4);
        cudaMalloc(&g.d_row_ptrs,(g.R+1)*4); cudaMemcpy(g.d_row_ptrs,rp.data(),(g.R+1)*4,cudaMemcpyHostToDevice);
        uint32_t nnz=rp[g.R]; std::vector<uint32_t> ci(nnz); sf.read((char*)ci.data(),nnz*4);
        cudaMalloc(&g.d_col_indices,nnz*4); cudaMemcpy(g.d_col_indices,ci.data(),nnz*4,cudaMemcpyHostToDevice);
    }
    g_ctx.cached_srs_path = srs_path;
}

extern "C" void gpu_full_commit(
    const uint32_t* h_packed_evals,
    uint32_t commit_len, uint32_t msg_len, uint32_t cw_len,
    const char* srs_path,
    uint8_t* h_leaf_hashes, uint8_t* h_nodes,
    uint8_t* h_leaves_raw,
    uint32_t* out_n_leaves)
{
    if (!g_ctx.initialized) {
        g_ctx.d_buffer = g_ctx.d_scratch = g_ctx.d_transposed = nullptr;
        g_ctx.d_lh = g_ctx.d_nd = nullptr;
        g_ctx.buf_cap = g_ctx.scratch_cap = g_ctx.trans_cap = 0;
        g_ctx.lh_cap = g_ctx.nd_cap = 0;
        g_ctx.initialized = true;
    }

    uint32_t packed_rows = commit_len / msg_len;
    load_srs(srs_path);

    // Allocate batched encode buffer: packed_rows × cw_len × 16 uint32_t
    size_t buf_bytes = (size_t)packed_rows * cw_len * 16 * 4;
    ensure_u32(&g_ctx.d_buffer, &g_ctx.buf_cap, buf_bytes);
    ensure_u32(&g_ctx.d_scratch, &g_ctx.scratch_cap, buf_bytes);

    // Upload ALL rows at once (message portion), zero the rest
    cudaMemset(g_ctx.d_buffer, 0, buf_bytes);
    // Copy each row's message into the buffer at the right offset
    for (uint32_t r = 0; r < packed_rows; r++) {
        cudaMemcpy(
            g_ctx.d_buffer + (uint64_t)r * cw_len * 16,
            h_packed_evals + (uint64_t)r * msg_len * 16,
            (size_t)msg_len * 16 * 4,
            cudaMemcpyHostToDevice);
    }

    // Batched Spielman encode: ALL rows in single kernel launches
    gpu_spielman_encode_m31x16(
        g_ctx.d_buffer, g_ctx.d_scratch,
        packed_rows, cw_len,
        g_ctx.graphs.data(), g_ctx.n_graphs);

    // Transpose: [packed_rows × cw_len] → [cw_len × packed_rows] (in M31x16 elements)
    uint64_t total_m31x16 = (uint64_t)packed_rows * cw_len;
    uint64_t padded = 1; while (padded < total_m31x16) padded <<= 1;
    size_t trans_bytes = padded * 64;
    ensure_u32(&g_ctx.d_transposed, &g_ctx.trans_cap, trans_bytes);
    if (padded > total_m31x16) {
        cudaMemset(g_ctx.d_transposed, 0, trans_bytes);
    }
    {
        uint32_t total = packed_rows * cw_len * 16;
        kernel_transpose_m31x16<<<(total+255)/256,256>>>(
            g_ctx.d_buffer, g_ctx.d_transposed, packed_rows, cw_len);
    }

    // Merkle tree
    uint32_t n_leaves = (uint32_t)padded;
    ensure_u8(&g_ctx.d_lh, &g_ctx.lh_cap, (size_t)n_leaves * 32);
    ensure_u8(&g_ctx.d_nd, &g_ctx.nd_cap, (size_t)(n_leaves > 0 ? n_leaves - 1 : 0) * 32);

    k_hash_leaf<<<(n_leaves+255)/256,256>>>((uint8_t*)g_ctx.d_transposed, g_ctx.d_lh, n_leaves);
    if (n_leaves >= 2) {
        uint32_t ls = n_leaves/2, st = ls-1;
        k_hash_pair<<<(ls+255)/256,256>>>(g_ctx.d_lh, g_ctx.d_nd+(uint64_t)st*32, ls);
        while (ls > 1) { ls/=2; uint32_t ps=ls-1,cs=2*ls-1;
            k_hash_pair<<<(ls+255)/256,256>>>(g_ctx.d_nd+(uint64_t)cs*32, g_ctx.d_nd+(uint64_t)ps*32, ls); }
    }
    cudaDeviceSynchronize();

    // Download
    cudaMemcpy(h_leaves_raw, g_ctx.d_transposed, (size_t)n_leaves*64, cudaMemcpyDeviceToHost);
    cudaMemcpy(h_leaf_hashes, g_ctx.d_lh, (size_t)n_leaves*32, cudaMemcpyDeviceToHost);
    cudaMemcpy(h_nodes, g_ctx.d_nd, (size_t)(n_leaves-1)*32, cudaMemcpyDeviceToHost);
    *out_n_leaves = n_leaves;
}
