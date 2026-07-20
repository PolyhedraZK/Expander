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

#include <map>

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

// ---- SRS cache + work buffer context ----
// One loaded SRS (expander graphs, VRAM-resident). Small (CSR of sparse graphs),
// so we keep every distinct one resident in a map keyed by its file path. Lowering
// the GPU-commit threshold means many distinct column shapes hit the GPU in one run;
// a single-entry cache would thrash (reload+H2D per size switch, ~2-7ms each).
struct SrsEntry {
    std::vector<ExpanderGraphGpu> graphs;
    uint32_t n_graphs;
};
static struct {
    std::map<std::string, SrsEntry> cache;
    SrsEntry* cur;              // entry for the most recent load_srs()
    uint32_t *d_buffer, *d_scratch;
    size_t buf_cap, scratch_cap;
    bool initialized;
} g_srs = {.cur = nullptr, .initialized = false};

static void ensure_u32(uint32_t** p, size_t* cap, size_t needed) {
    if (*cap >= needed) return;
    if (*p) cudaFree(*p);
    cudaMalloc(p, needed); *cap = needed;
}

static void load_srs(const char* srs_path) {
    auto it = g_srs.cache.find(srs_path);
    if (it != g_srs.cache.end()) { g_srs.cur = &it->second; return; }
    SrsEntry& e = g_srs.cache[srs_path];
    std::ifstream sf(srs_path, std::ios::binary);
    uint32_t s_ml, s_cw, s_ng, s_nlpq;
    sf.read((char*)&s_ml,4); sf.read((char*)&s_cw,4); sf.read((char*)&s_ng,4); sf.read((char*)&s_nlpq,4);
    e.graphs.resize(s_ng); e.n_graphs = s_ng;
    for (uint32_t gi = 0; gi < s_ng; gi++) {
        auto& g = e.graphs[gi];
        sf.read((char*)&g.input_start,4); sf.read((char*)&g.output_start,4); sf.read((char*)&g.output_end,4);
        g.R = g.output_end - g.output_start + 1;
        std::vector<uint32_t> rp(g.R+1); sf.read((char*)rp.data(),(g.R+1)*4);
        cudaMalloc(&g.d_row_ptrs,(g.R+1)*4); cudaMemcpy(g.d_row_ptrs,rp.data(),(g.R+1)*4,cudaMemcpyHostToDevice);
        uint32_t nnz=rp[g.R]; std::vector<uint32_t> ci(nnz); sf.read((char*)ci.data(),nnz*4);
        cudaMalloc(&g.d_col_indices,nnz*4); cudaMemcpy(g.d_col_indices,ci.data(),nnz*4,cudaMemcpyHostToDevice);
    }
    g_srs.cur = &e;
}

// ---- GPU-resident tree slots ----
#define MAX_GPU_TREES 32
static struct GpuTreeSlot {
    uint32_t* d_leaves;  // raw transposed data (n_leaves × 16 uint32_t = 64 bytes each)
    uint8_t* d_lh;       // leaf hashes (n_leaves × 32)
    uint8_t* d_nd;       // internal nodes ((n_leaves-1) × 32)
    uint32_t* d_poly;    // original polynomial (commit_len × 16 uint32_t, before encoding)
    uint32_t commit_len; // original polynomial size
    uint32_t msg_len;    // message length (per row)
    const uint32_t* h_poly_ptr; // host pointer for matching in gpu_tree_find_poly
    uint32_t n_leaves;
    bool active;
} g_trees[MAX_GPU_TREES] = {};

extern "C" void gpu_preload_srs(const char* srs_path) {
    if (!g_srs.initialized) {
        g_srs.d_buffer = g_srs.d_scratch = nullptr;
        g_srs.buf_cap = g_srs.scratch_cap = 0;
        g_srs.initialized = true;
    }
    load_srs(srs_path);
}

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
    auto tc = std::chrono::high_resolution_clock::now;
    auto t0 = tc();
    load_srs(srs_path);
    auto t_srs = tc();

    // Work buffers (reused across commits)
    size_t buf_bytes = (size_t)packed_rows * cw_len * 16 * 4;
    ensure_u32(&g_srs.d_buffer, &g_srs.buf_cap, buf_bytes);
    ensure_u32(&g_srs.d_scratch, &g_srs.scratch_cap, buf_bytes);
    cudaMemset(g_srs.d_buffer, 0, buf_bytes);

    // Upload polynomial + scatter to strided encode buffer in one step
    size_t poly_bytes = (size_t)commit_len * 16 * 4;
    auto& tree = g_trees[slot];
    tree.h_poly_ptr = h_packed_evals;
    cudaMalloc(&tree.d_poly, poly_bytes);
    // Bulk upload to d_poly (contiguous), then scatter to strided d_buffer
    cudaMemcpy(tree.d_poly, h_packed_evals, poly_bytes, cudaMemcpyHostToDevice);
    // Scatter: d_poly[r*msg_len*16 .. (r+1)*msg_len*16] → d_buffer[r*cw_len*16 .. r*cw_len*16 + msg_len*16]
    for (uint32_t r = 0; r < packed_rows; r++) {
        cudaMemcpy(g_srs.d_buffer + (uint64_t)r * cw_len * 16,
                   tree.d_poly + (uint64_t)r * msg_len * 16,
                   (size_t)msg_len * 16 * 4, cudaMemcpyDeviceToDevice);
    }
    tree.commit_len = commit_len;
    tree.msg_len = msg_len;
    auto t_upload = tc();
    auto t_poly = tc();

    // Batched Spielman encode
    gpu_spielman_encode_m31x16(g_srs.d_buffer, g_srs.d_scratch,
        packed_rows, cw_len, g_srs.cur->graphs.data(), g_srs.cur->n_graphs);
    cudaDeviceSynchronize();
    auto t_encode = tc();

    // Transpose into NEW persistent buffer (this tree's leaves)
    uint64_t total_m31x16 = (uint64_t)packed_rows * cw_len;
    uint64_t padded = 1; while (padded < total_m31x16) padded <<= 1;
    uint32_t n_leaves = (uint32_t)padded;

    cudaMalloc(&tree.d_leaves, (size_t)n_leaves * 64);
    if (padded > total_m31x16) cudaMemset(tree.d_leaves, 0, (size_t)n_leaves * 64);
    {
        uint32_t total = packed_rows * cw_len * 16;
        kernel_transpose_m31x16<<<(total+255)/256,256>>>(
            g_srs.d_buffer, tree.d_leaves, packed_rows, cw_len);
    }
    cudaDeviceSynchronize();
    auto t_transpose = tc();

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
    auto t_tree = tc();

    // Download ONLY root hash (32 bytes)
    cudaMemcpy(h_root_hash, tree.d_nd, 32, cudaMemcpyDeviceToHost);
    auto us = [](auto a, auto b) { return std::chrono::duration_cast<std::chrono::microseconds>(b-a).count(); };
    fprintf(stderr, "    [gpu-commit-detail] %u leaves: srs=%ldus upload=%ldus poly=%ldus encode=%ldus transpose=%ldus tree=%ldus total=%ldus\n",
        n_leaves, us(t0,t_srs), us(t_srs,t_upload), us(t_upload,t_poly), us(t_poly,t_encode),
        us(t_encode,t_transpose), us(t_transpose,t_tree), us(t0,t_tree));
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
    if (t.d_poly) cudaFree(t.d_poly);
    t.d_leaves = nullptr; t.d_lh = nullptr; t.d_nd = nullptr; t.d_poly = nullptr;
    t.active = false;
}

// Batch extract range queries from GPU-resident tree.
// For each query: extracts leaf data + Merkle proof (sibling hashes).
// Output format per query: [leaf_data (range_size × 64 bytes)] [sibling_hashes (depth × 32 bytes)]
extern "C" void gpu_tree_range_queries(
    int32_t tree_id,
    const uint32_t* h_query_ranges, // [left_0, right_0, left_1, right_1, ...]
    uint32_t n_queries,
    uint32_t leaves_per_query,      // right - left + 1 (same for all queries)
    uint8_t* h_leaf_data_out,       // n_queries × leaves_per_query × 64
    uint8_t* h_sibling_hashes_out,  // n_queries × depth × 32  (depth = log2(n_leaves/leaves_per_query))
    uint32_t* out_depth)
{
    if (tree_id < 0 || tree_id >= MAX_GPU_TREES || !g_trees[tree_id].active) return;
    auto& t = g_trees[tree_id];
    uint32_t n = t.n_leaves;
    uint32_t depth = 0; { uint32_t x = n / leaves_per_query; while (x > 1) { depth++; x >>= 1; } }
    *out_depth = depth;

    // Upload query indices
    uint32_t* d_queries;
    cudaMalloc(&d_queries, n_queries * 2 * 4);
    cudaMemcpy(d_queries, h_query_ranges, n_queries * 2 * 4, cudaMemcpyHostToDevice);

    // Allocate output buffers on GPU
    size_t leaf_out_size = (size_t)n_queries * leaves_per_query * 64;
    size_t sibling_out_size = (size_t)n_queries * depth * 32;
    uint8_t *d_leaf_out, *d_sibling_out;
    cudaMalloc(&d_leaf_out, leaf_out_size);
    cudaMalloc(&d_sibling_out, sibling_out_size);

    // Extract leaf data: for each query, copy leaves[left..right] raw data
    // Simple approach: download specific ranges from d_leaves
    // Since queries are random, do batch cudaMemcpy per query (or use a kernel)
    for (uint32_t qi = 0; qi < n_queries; qi++) {
        uint32_t left = h_query_ranges[qi*2];
        cudaMemcpy(d_leaf_out + (uint64_t)qi * leaves_per_query * 64,
                   (uint8_t*)t.d_leaves + (uint64_t)left * 64,
                   leaves_per_query * 64, cudaMemcpyDeviceToDevice);
    }

    // Extract sibling hashes: walk up the tree for each query
    // nodes[0] = root, nodes[n-2..0] = bottom-up internal nodes
    // Leaf hashes at level 0: d_lh[i*32..(i+1)*32]
    // Parent of leaves[left..right] at level 0: node at index (n/2 - 1) + left/leaves_per_query
    // Sibling: adjacent node at same level
    for (uint32_t qi = 0; qi < n_queries; qi++) {
        uint32_t left = h_query_ranges[qi*2];
        uint32_t idx = left / leaves_per_query; // index within this level
        // Level 0: siblings from leaf hashes (d_lh)
        // But range query covers leaves_per_query leaves which form a subtree
        // The sibling at each level is the adjacent subtree hash
        uint32_t level_size = n / leaves_per_query;
        uint32_t level_start = level_size - 1; // start of this level in nodes array
        for (uint32_t d = 0; d < depth; d++) {
            uint32_t sibling = idx ^ 1;
            if (level_size > 1) {
                // Internal node: read from d_nd at position level_start + sibling
                cudaMemcpy(d_sibling_out + ((uint64_t)qi * depth + d) * 32,
                           t.d_nd + (uint64_t)(level_start + sibling) * 8, // 32 bytes = 8 uint32_t
                           32, cudaMemcpyDeviceToDevice);
            }
            idx >>= 1;
            level_size >>= 1;
            level_start = level_size - 1;
        }
    }
    cudaDeviceSynchronize();

    // Download results
    cudaMemcpy(h_leaf_data_out, d_leaf_out, leaf_out_size, cudaMemcpyDeviceToHost);
    cudaMemcpy(h_sibling_hashes_out, d_sibling_out, sibling_out_size, cudaMemcpyDeviceToHost);

    cudaFree(d_queries); cudaFree(d_leaf_out); cudaFree(d_sibling_out);
}

extern "C" void gpu_tree_get_ptrs(int32_t tree_id,
    uint32_t** out_leaves, uint8_t** out_lh, uint8_t** out_nd, uint32_t* out_n,
    uint32_t** out_poly, uint32_t* out_commit_len, uint32_t* out_msg_len) {
    if (tree_id < 0 || tree_id >= MAX_GPU_TREES || !g_trees[tree_id].active) {
        *out_leaves = nullptr; *out_lh = nullptr; *out_nd = nullptr; *out_n = 0;
        *out_poly = nullptr; *out_commit_len = 0; *out_msg_len = 0; return;
    }
    auto& t = g_trees[tree_id];
    *out_leaves = t.d_leaves; *out_lh = t.d_lh; *out_nd = t.d_nd; *out_n = t.n_leaves;
    *out_poly = t.d_poly; *out_commit_len = t.commit_len; *out_msg_len = t.msg_len;
}

// Find d_poly for a host pointer. Matches by exact host address.
extern "C" const uint32_t* gpu_tree_find_poly(const uint32_t* h_ptr, uint32_t commit_len) {
    for (int i = 0; i < MAX_GPU_TREES; i++) {
        if (g_trees[i].active && g_trees[i].h_poly_ptr == h_ptr && g_trees[i].d_poly) {
            return g_trees[i].d_poly;
        }
    }
    return nullptr;
}

extern "C" void gpu_tree_free_all() {
    for (int i = 0; i < MAX_GPU_TREES; i++) gpu_tree_free(i);
}

