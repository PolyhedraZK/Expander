// gpu_commit.cu — Full GPU PCS commit: Spielman encode + transpose + Blake3 Merkle
// Streaming row-by-row to avoid OOM. Data stays on GPU (no PCIe for tree).
//
// extern "C" void gpu_full_commit(
//     const uint32_t* h_packed_evals, // commit_len * 16 uint32_t
//     uint32_t commit_len, uint32_t msg_len, uint32_t cw_len,
//     const char* srs_path,          // path to SRS binary file
//     uint8_t* h_leaf_hashes,        // n_leaves * 32 bytes (output)
//     uint8_t* h_nodes,              // (n_leaves-1) * 32 bytes (output)
//     uint32_t* out_n_leaves         // actual n_leaves (output)
// );

#include <cstdint>
#include <cstdio>
#include <vector>
#include <fstream>

static constexpr uint32_t M31_MOD = (1u << 31) - 1;

// SpMV kernel for Spielman encode (one row at a time, M31x16)
__global__ void k_spmv_row(
    const uint32_t* row_ptrs, const uint32_t* col_indices,
    const uint32_t* buffer, uint32_t* scratch,
    uint32_t input_offset, uint32_t output_offset,
    uint32_t R, uint32_t cw_len)
{
    uint32_t idx = blockIdx.x * blockDim.x + threadIdx.x;
    uint32_t total = R * 16;
    if (idx >= total) return;
    uint32_t lane = idx % 16, r = idx / 16;

    uint32_t start = row_ptrs[r], end = row_ptrs[r+1];
    uint32_t sum = 0;
    for (uint32_t j = start; j < end; j++) {
        uint32_t l = col_indices[j];
        sum += buffer[(uint64_t)(input_offset + l) * 16 + lane];
        if (sum >= M31_MOD) sum -= M31_MOD;
    }
    sum = (sum & M31_MOD) + (sum >> 31);
    if (sum >= M31_MOD) sum -= M31_MOD;
    scratch[(uint64_t)(output_offset + r) * 16 + lane] = sum;
}

__global__ void k_copy_seg(const uint32_t* src, uint32_t* dst,
    uint32_t offset, uint32_t R) {
    uint32_t idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= R * 16) return;
    uint32_t r = idx / 16, lane = idx % 16;
    dst[(uint64_t)(offset + r) * 16 + lane] = src[(uint64_t)(offset + r) * 16 + lane];
}

// Write one encoded row into transposed buffer at correct positions
// transposed[col][row] where col=0..cw_len-1, row=row_idx
// Each element is M31x16 = 16 uint32_t
// Source: encoded_row[elem_idx * 16 + lane]
// Dest: transposed[(elem_idx * packed_rows + row_idx) * 16 + lane]
__global__ void k_scatter_transpose(
    const uint32_t* encoded_row, uint32_t* transposed,
    uint32_t cw_len, uint32_t packed_rows, uint32_t row_idx)
{
    uint32_t idx = blockIdx.x * blockDim.x + threadIdx.x;
    uint32_t total = cw_len * 16;
    if (idx >= total) return;
    uint32_t elem = idx / 16, lane = idx % 16;
    transposed[((uint64_t)elem * packed_rows + row_idx) * 16 + lane] = encoded_row[(uint64_t)elem * 16 + lane];
}

// Blake3 (same as in pcs_linear_combine.cu — duplicated to keep files independent)
__device__ static inline uint32_t rotr(uint32_t x,int n){return(x>>n)|(x<<(32-n));}
__device__ static void b3g(uint32_t*s,int a,int b,int c,int d,uint32_t mx,uint32_t my){
    s[a]=s[a]+s[b]+mx;s[d]=rotr(s[d]^s[a],16);s[c]=s[c]+s[d];s[b]=rotr(s[b]^s[c],12);
    s[a]=s[a]+s[b]+my;s[d]=rotr(s[d]^s[a],8);s[c]=s[c]+s[d];s[b]=rotr(s[b]^s[c],7);
}
__device__ static void b3round(uint32_t*s,const uint32_t*m){
    b3g(s,0,4,8,12,m[0],m[1]);b3g(s,1,5,9,13,m[2],m[3]);b3g(s,2,6,10,14,m[4],m[5]);b3g(s,3,7,11,15,m[6],m[7]);
    b3g(s,0,5,10,15,m[8],m[9]);b3g(s,1,6,11,12,m[10],m[11]);b3g(s,2,7,8,13,m[12],m[13]);b3g(s,3,4,9,14,m[14],m[15]);
}
__device__ static void b3perm(uint32_t*m){
    uint32_t t[16]={m[2],m[6],m[3],m[10],m[7],m[0],m[4],m[13],m[1],m[11],m[12],m[5],m[9],m[14],m[15],m[8]};
    for(int i=0;i<16;i++)m[i]=t[i];
}
__device__ static void b3hash64(const uint8_t*data,uint8_t*out){
    uint32_t iv[8]={0x6A09E667,0xBB67AE85,0x3C6EF372,0xA54FF53A,0x510E527F,0x9B05688C,0x1F83D9AB,0x5BE0CD19};
    uint32_t blk[16];for(int i=0;i<16;i++)blk[i]=(uint32_t)data[4*i]|((uint32_t)data[4*i+1]<<8)|((uint32_t)data[4*i+2]<<16)|((uint32_t)data[4*i+3]<<24);
    uint32_t s[16]={iv[0],iv[1],iv[2],iv[3],iv[4],iv[5],iv[6],iv[7],iv[0],iv[1],iv[2],iv[3],0,0,64,0x0B};
    uint32_t m[16];for(int i=0;i<16;i++)m[i]=blk[i];
    b3round(s,m);b3perm(m);b3round(s,m);b3perm(m);b3round(s,m);b3perm(m);
    b3round(s,m);b3perm(m);b3round(s,m);b3perm(m);b3round(s,m);b3perm(m);b3round(s,m);
    for(int i=0;i<8;i++){uint32_t v=s[i]^s[i+8];out[4*i]=(uint8_t)v;out[4*i+1]=(uint8_t)(v>>8);out[4*i+2]=(uint8_t)(v>>16);out[4*i+3]=(uint8_t)(v>>24);}
}
__global__ void k_hash_l(const uint8_t*data,uint8_t*hashes,uint32_t n){
    uint32_t i=blockIdx.x*blockDim.x+threadIdx.x;if(i>=n)return;
    b3hash64(data+(uint64_t)i*64,hashes+(uint64_t)i*32);
}
__global__ void k_hash_p(const uint8_t*c,uint8_t*p,uint32_t n){
    uint32_t i=blockIdx.x*blockDim.x+threadIdx.x;if(i>=n)return;
    uint8_t buf[64];for(int b=0;b<32;b++){buf[b]=c[(uint64_t)i*64+b];buf[32+b]=c[(uint64_t)i*64+32+b];}
    b3hash64(buf,p+(uint64_t)i*32);
}

struct GraphGpu {
    uint32_t input_start, output_start, output_end, R;
    uint32_t *d_rp, *d_ci;
};

extern "C" void gpu_full_commit(
    const uint32_t* h_packed_evals,
    uint32_t commit_len, uint32_t msg_len, uint32_t cw_len,
    const char* srs_path,
    uint8_t* h_leaf_hashes, uint8_t* h_nodes,
    uint8_t* h_leaves_raw,  // n_leaves * 64 bytes (raw leaf data for Tree)
    uint32_t* out_n_leaves)
{
    uint32_t packed_rows = commit_len / msg_len;

    // Load SRS
    std::ifstream sf(srs_path, std::ios::binary);
    uint32_t s_ml, s_cw, s_ng, s_nlpq;
    sf.read((char*)&s_ml,4); sf.read((char*)&s_cw,4); sf.read((char*)&s_ng,4); sf.read((char*)&s_nlpq,4);
    std::vector<GraphGpu> graphs(s_ng);
    for (uint32_t gi = 0; gi < s_ng; gi++) {
        auto& g = graphs[gi];
        sf.read((char*)&g.input_start,4); sf.read((char*)&g.output_start,4); sf.read((char*)&g.output_end,4);
        g.R = g.output_end - g.output_start + 1;
        std::vector<uint32_t> rp(g.R+1); sf.read((char*)rp.data(),(g.R+1)*4);
        cudaMalloc(&g.d_rp,(g.R+1)*4); cudaMemcpy(g.d_rp,rp.data(),(g.R+1)*4,cudaMemcpyHostToDevice);
        uint32_t nnz=rp[g.R]; std::vector<uint32_t> ci(nnz); sf.read((char*)ci.data(),nnz*4);
        cudaMalloc(&g.d_ci,nnz*4); cudaMemcpy(g.d_ci,ci.data(),nnz*4,cudaMemcpyHostToDevice);
    }

    // Allocate transposed buffer (padded)
    uint64_t total_m31x16 = (uint64_t)packed_rows * cw_len;
    uint64_t padded = 1; while (padded < total_m31x16) padded <<= 1;
    uint32_t* d_transposed; cudaMalloc(&d_transposed, padded * 64);
    cudaMemset(d_transposed, 0, padded * 64);

    // Per-row encode buffer + scratch
    uint32_t* d_row_buf; cudaMalloc(&d_row_buf, (size_t)cw_len * 16 * 4);
    uint32_t* d_row_scratch; cudaMalloc(&d_row_scratch, (size_t)cw_len * 16 * 4);

    // Stream encode row by row
    for (uint32_t r = 0; r < packed_rows; r++) {
        // Upload one row's message data
        cudaMemset(d_row_buf, 0, (size_t)cw_len * 16 * 4);
        cudaMemcpy(d_row_buf, h_packed_evals + (uint64_t)r * msg_len * 16,
                   (size_t)msg_len * 16 * 4, cudaMemcpyHostToDevice);

        // Spielman encode
        for (uint32_t gi = 0; gi < s_ng; gi++) {
            auto& g = graphs[gi];
            uint32_t total = g.R * 16;
            k_spmv_row<<<(total+255)/256,256>>>(g.d_rp, g.d_ci, d_row_buf, d_row_scratch,
                g.input_start, g.output_start, g.R, cw_len);
            k_copy_seg<<<(total+255)/256,256>>>(d_row_scratch, d_row_buf, g.output_start, g.R);
        }

        // Scatter to transposed buffer
        uint32_t total = cw_len * 16;
        k_scatter_transpose<<<(total+255)/256,256>>>(d_row_buf, d_transposed, cw_len, packed_rows, r);
    }
    cudaFree(d_row_buf); cudaFree(d_row_scratch);

    // Merkle tree (data already on GPU!)
    uint32_t n_leaves = (uint32_t)padded;
    uint8_t* d_lh; cudaMalloc(&d_lh, (size_t)n_leaves * 32);
    uint8_t* d_nd; cudaMalloc(&d_nd, (size_t)(n_leaves-1) * 32);

    k_hash_l<<<(n_leaves+255)/256,256>>>((uint8_t*)d_transposed, d_lh, n_leaves);
    uint32_t ls = n_leaves/2, st = ls-1;
    k_hash_p<<<(ls+255)/256,256>>>(d_lh, d_nd+(uint64_t)st*32, ls);
    while (ls > 1) { ls/=2; uint32_t ps=ls-1,cs=2*ls-1;
        k_hash_p<<<(ls+255)/256,256>>>(d_nd+(uint64_t)cs*32, d_nd+(uint64_t)ps*32, ls); }
    cudaDeviceSynchronize();

    // Download results
    cudaMemcpy(h_leaves_raw, d_transposed, (size_t)n_leaves*64, cudaMemcpyDeviceToHost);
    cudaMemcpy(h_leaf_hashes, d_lh, (size_t)n_leaves*32, cudaMemcpyDeviceToHost);
    cudaMemcpy(h_nodes, d_nd, (size_t)(n_leaves-1)*32, cudaMemcpyDeviceToHost);
    *out_n_leaves = n_leaves;

    // Cleanup
    cudaFree(d_transposed); cudaFree(d_lh); cudaFree(d_nd);
    for (auto& g : graphs) { cudaFree(g.d_rp); cudaFree(g.d_ci); }
}
