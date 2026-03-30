// gpu_commit.cu — GPU PCS commit with VRAM-resident trees.
// Encode + transpose + Merkle on GPU. Tree stays in GPU memory.
// Only root hash downloaded during commit. Full tree downloaded lazily on demand.

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <vector>
#include <fstream>
#include <string>
#include <chrono>

#include "spielman.cuh"
#include "blake3.cuh"

__global__ void k_hash_leaf(const uint8_t*data,uint8_t*hashes,uint32_t n){
    uint32_t i=blockIdx.x*blockDim.x+threadIdx.x;if(i>=n)return;
    blake3_hash_64(data+(uint64_t)i*64,hashes+(uint64_t)i*32);
}
__global__ void k_hash_pair(const uint8_t*children,uint8_t*parents,uint32_t n){
    uint32_t i=blockIdx.x*blockDim.x+threadIdx.x;if(i>=n)return;
    uint8_t buf[64];
    for(int b=0;b<32;b++){buf[b]=children[(uint64_t)i*64+b];buf[32+b]=children[(uint64_t)i*64+32+b];}
    blake3_hash_64(buf,parents+(uint64_t)i*32);
}

// ---- SRS + work buffer context ----
static struct {
    std::string cached_srs_path;
    std::vector<ExpanderGraphGpu> graphs;
    uint32_t n_graphs;
    uint32_t *d_buffer, *d_scratch;
    size_t buf_cap, scratch_cap;
    bool initialized;
} g_srs = {.initialized = false};

static void ensure_u32(uint32_t** p, size_t* cap, size_t needed) {
    if (*cap >= needed) return;
    if (*p) cudaFree(*p);
    cudaMalloc(p, needed); *cap = needed;
}

static void load_srs(const char* srs_path) {
    if (g_srs.cached_srs_path == srs_path) return;
    for (auto& g : g_srs.graphs) { cudaFree(g.d_row_ptrs); cudaFree(g.d_col_indices); }
    g_srs.graphs.clear();
    std::ifstream sf(srs_path, std::ios::binary);
    uint32_t s_ml, s_cw, s_ng, s_nlpq;
    sf.read((char*)&s_ml,4); sf.read((char*)&s_cw,4); sf.read((char*)&s_ng,4); sf.read((char*)&s_nlpq,4);
    g_srs.graphs.resize(s_ng); g_srs.n_graphs = s_ng;
    for (uint32_t gi = 0; gi < s_ng; gi++) {
        auto& g = g_srs.graphs[gi];
        sf.read((char*)&g.input_start,4); sf.read((char*)&g.output_start,4); sf.read((char*)&g.output_end,4);
        g.R = g.output_end - g.output_start + 1;
        std::vector<uint32_t> rp(g.R+1); sf.read((char*)rp.data(),(g.R+1)*4);
        cudaMalloc(&g.d_row_ptrs,(g.R+1)*4); cudaMemcpy(g.d_row_ptrs,rp.data(),(g.R+1)*4,cudaMemcpyHostToDevice);
        uint32_t nnz=rp[g.R]; std::vector<uint32_t> ci(nnz); sf.read((char*)ci.data(),nnz*4);
        cudaMalloc(&g.d_col_indices,nnz*4); cudaMemcpy(g.d_col_indices,ci.data(),nnz*4,cudaMemcpyHostToDevice);
    }
    g_srs.cached_srs_path = srs_path;
}

// ---- GPU-resident tree slots ----
#define MAX_GPU_TREES 32
static struct GpuTreeSlot {
    uint32_t* d_leaves;  // raw transposed data (n_leaves × 16 uint32_t = 64 bytes each)
    uint8_t* d_lh;       // leaf hashes (n_leaves × 32)
    uint8_t* d_nd;       // internal nodes ((n_leaves-1) × 32)
    uint32_t n_leaves;
    bool active;
} g_trees[MAX_GPU_TREES] = {};

extern "C" int32_t gpu_commit_to_tree(
    const uint32_t* h_packed_evals,
    uint32_t commit_len, uint32_t msg_len, uint32_t cw_len,
    const char* srs_path,
    uint8_t* h_root_hash,    // output: 32 bytes root
    uint32_t* out_n_leaves)
{
    if (!g_srs.initialized) {
        g_srs.d_buffer = g_srs.d_scratch = nullptr;
        g_srs.buf_cap = g_srs.scratch_cap = 0;
        g_srs.initialized = true;
    }

    // Find free slot
    int32_t slot = -1;
    for (int i = 0; i < MAX_GPU_TREES; i++) {
        if (!g_trees[i].active) { slot = i; break; }
    }
    if (slot < 0) { fprintf(stderr, "gpu_commit: no free tree slots\n"); return -1; }

    uint32_t packed_rows = commit_len / msg_len;
    load_srs(srs_path);

    // Work buffers (reused across commits)
    size_t buf_bytes = (size_t)packed_rows * cw_len * 16 * 4;
    ensure_u32(&g_srs.d_buffer, &g_srs.buf_cap, buf_bytes);
    ensure_u32(&g_srs.d_scratch, &g_srs.scratch_cap, buf_bytes);
    cudaMemset(g_srs.d_buffer, 0, buf_bytes);

    // Upload rows
    for (uint32_t r = 0; r < packed_rows; r++) {
        cudaMemcpy(g_srs.d_buffer + (uint64_t)r * cw_len * 16,
                   h_packed_evals + (uint64_t)r * msg_len * 16,
                   (size_t)msg_len * 16 * 4, cudaMemcpyHostToDevice);
    }

    // Batched Spielman encode
    gpu_spielman_encode_m31x16(g_srs.d_buffer, g_srs.d_scratch,
        packed_rows, cw_len, g_srs.graphs.data(), g_srs.n_graphs);

    // Transpose into NEW persistent buffer (this tree's leaves)
    uint64_t total_m31x16 = (uint64_t)packed_rows * cw_len;
    uint64_t padded = 1; while (padded < total_m31x16) padded <<= 1;
    uint32_t n_leaves = (uint32_t)padded;

    auto& tree = g_trees[slot];
    cudaMalloc(&tree.d_leaves, (size_t)n_leaves * 64);
    if (padded > total_m31x16) cudaMemset(tree.d_leaves, 0, (size_t)n_leaves * 64);
    {
        uint32_t total = packed_rows * cw_len * 16;
        kernel_transpose_m31x16<<<(total+255)/256,256>>>(
            g_srs.d_buffer, tree.d_leaves, packed_rows, cw_len);
    }

    // Merkle tree into persistent buffers
    cudaMalloc(&tree.d_lh, (size_t)n_leaves * 32);
    cudaMalloc(&tree.d_nd, (size_t)(n_leaves - 1) * 32);
    tree.n_leaves = n_leaves;

    k_hash_leaf<<<(n_leaves+255)/256,256>>>((uint8_t*)tree.d_leaves, tree.d_lh, n_leaves);
    uint32_t ls = n_leaves/2, st = ls-1;
    k_hash_pair<<<(ls+255)/256,256>>>(tree.d_lh, tree.d_nd+(uint64_t)st*32, ls);
    while (ls > 1) { ls/=2; uint32_t ps=ls-1,cs=2*ls-1;
        k_hash_pair<<<(ls+255)/256,256>>>(tree.d_nd+(uint64_t)cs*32, tree.d_nd+(uint64_t)ps*32, ls); }
    cudaDeviceSynchronize();

    // Download ONLY root hash (32 bytes)
    cudaMemcpy(h_root_hash, tree.d_nd, 32, cudaMemcpyDeviceToHost);
    *out_n_leaves = n_leaves;
    tree.active = true;

    return slot;
}

// Download full tree from GPU (for PCS opening compatibility)
extern "C" void gpu_tree_download(
    int32_t tree_id,
    uint8_t* h_leaf_hashes,
    uint8_t* h_nodes,
    uint8_t* h_leaves_raw)
{
    if (tree_id < 0 || tree_id >= MAX_GPU_TREES || !g_trees[tree_id].active) return;
    auto& t = g_trees[tree_id];
    cudaMemcpy(h_leaves_raw, t.d_leaves, (size_t)t.n_leaves * 64, cudaMemcpyDeviceToHost);
    cudaMemcpy(h_leaf_hashes, t.d_lh, (size_t)t.n_leaves * 32, cudaMemcpyDeviceToHost);
    cudaMemcpy(h_nodes, t.d_nd, (size_t)(t.n_leaves - 1) * 32, cudaMemcpyDeviceToHost);
}

extern "C" void gpu_tree_free(int32_t tree_id) {
    if (tree_id < 0 || tree_id >= MAX_GPU_TREES || !g_trees[tree_id].active) return;
    auto& t = g_trees[tree_id];
    cudaFree(t.d_leaves); cudaFree(t.d_lh); cudaFree(t.d_nd);
    t.d_leaves = nullptr; t.d_lh = nullptr; t.d_nd = nullptr;
    t.active = false;
}

extern "C" void gpu_tree_free_all() {
    for (int i = 0; i < MAX_GPU_TREES; i++) gpu_tree_free(i);
}

