use arith::{ExtensionField, Field, SimdField};
use gkr_engine::Transcript;
use itertools::izip;
use serdes::{ExpSerde, SerdeError};
use thiserror::Error;
use transpose::{transpose, transpose_inplace};
use tree::{Node, LEAF_BYTES};

use crate::{
    orion::linear_code::{OrionCode, OrionCodeParameter, ORION_CODE_PARAMETER_INSTANCE},
    traits::TensorCodeIOPPCS,
    PCS_SOUNDNESS_BITS,
};

/*
 * PCS ERROR AND RESULT SETUP
 */

#[derive(Debug, Error)]
pub enum OrionPCSError {
    #[error("Orion PCS linear code parameter unmatch error")]
    ParameterUnmatchError,

    #[error("field serde error")]
    SerializationError(#[from] SerdeError),
}

pub type OrionResult<T> = std::result::Result<T, OrionPCSError>;

/*
 * RELEVANT TYPES SETUP
 */

/// returning leaf number, calibrated local number of variables, and message size
/// NOTE(HS) num of local variables is over base field with no SIMD
pub(crate) const fn orion_eval_shape(
    world_size: usize,
    num_local_vars: usize,
    num_bits_base_field: usize,
    field_pack_size: usize,
) -> (usize, usize, usize) {
    // for a global polynomial, compute the boolean hypercube size
    let hypercube_size = world_size * (1 << num_local_vars);

    // compute how many bits the global polynomial occupy
    let polynomial_bits = hypercube_size * num_bits_base_field;
    let polynomial_bits_log2 = polynomial_bits.ilog2();

    // the leaf should be roughly O(\log N) to the global polynomial size
    let leaves_lower_bound = match polynomial_bits_log2 {
        // 128 bytes MT opening
        0..16 => 2,

        // 256 bytes MT opening
        16..22 => 4,

        // 512 bytes MT opening
        22..28 => 8,

        // 1 KB MT opening
        28..34 => 16,

        // 2 KB MT opening
        34.. => 32,
    };

    // now compute the commitment used SIMD field bits,
    // and compared against the suggest leaf size above.
    //
    // the commitment used SIMD field is the smallest unit of memory in commitment,
    // thus each world can contribute as low as 1 commitment used SIMD field element,
    // and thus if the world size is too large, we might need to scale up the final
    // leaf size based on suggested leaf size - and thus may require padding the poly
    // to the right size.
    let num_bits_packed_f = field_pack_size * num_bits_base_field;
    let minimum_pack_fs_per_world_in_query = {
        let leaves_bits_lower_bound = leaves_lower_bound * LEAF_BYTES * 8;
        // NOTE(HS) div_ceil ensures each world contribute at least 1 commitment SIMD field element
        // in the MT query opening
        leaves_bits_lower_bound.div_ceil(num_bits_packed_f * world_size)
    };

    let final_leaves_bits = minimum_pack_fs_per_world_in_query * world_size * num_bits_packed_f;
    let final_leaves_per_query = final_leaves_bits / (LEAF_BYTES * 8);

    // once we have decided the leaf size, we know the message size need to be encoded.
    // the consideration is that - resulting codeword size should be >= MPI world size.
    //
    // Otherwise, the MPI transpose cannot go through - the codeword is too short to be
    // segmented into the number of shares spreaded across MPI words for all-to-all transpose.
    let msg_size = polynomial_bits / final_leaves_bits;

    // we want to know the mininum message size, such that the resulting codeword length
    // (pad to next po2) >= the MPI world size.
    //
    // for small case, when input message is shorter than Expander code length threshold,
    // the code is itself (by Druk-Ishai),
    // otherwise Expander encoding applies, with a code rate > 1 / 2, eventually pad to 2x length.
    //
    // NOTE(HS): here we assume that the message size and the MPI world size are both po2,
    // while Expander code length threshold might be arbitarary - we scale up the threshold
    // to the next po2 to avoid corner case.
    let minimum_msg_size = {
        let min_expander_po2_code_len = ORION_CODE_PARAMETER_INSTANCE
            .length_threshold_g0s
            .next_power_of_two();

        if world_size <= min_expander_po2_code_len {
            world_size
        } else {
            world_size / 2
        }
    };

    // Early exit - if the message size from the global poly is sufficiently large,
    // we decide the global MT opening size, and are good with the local polynomial number of vars.
    if msg_size >= minimum_msg_size {
        return (final_leaves_per_query, num_local_vars, msg_size);
    }

    // Scale up the polynomial local number of variable and plan again for the evaluation shape.
    let scaled_up_num_local_vars = {
        let num_of_field_elems_per_world_in_query =
            minimum_pack_fs_per_world_in_query * field_pack_size;

        let minimum_poly_len = minimum_msg_size * num_of_field_elems_per_world_in_query;
        minimum_poly_len.ilog2() as usize
    };

    orion_eval_shape(
        world_size,
        scaled_up_num_local_vars,
        num_bits_base_field,
        field_pack_size,
    )
}

#[derive(Clone, Debug, Default, ExpSerde)]
pub struct OrionSRS {
    pub num_vars: usize,
    pub num_leaves_per_mt_query: usize,
    pub code_instance: OrionCode,
}

impl TensorCodeIOPPCS for OrionSRS {
    fn message_len(&self) -> usize {
        self.code_instance.msg_len()
    }

    fn codeword_len(&self) -> usize {
        self.code_instance.code_len()
    }

    fn minimum_hamming_weight(&self) -> f64 {
        self.code_instance.hamming_weight()
    }

    fn num_leaves_per_mt_query(&self) -> usize {
        self.num_leaves_per_mt_query
    }
}

impl OrionSRS {
    // NOTE(HS) num local variables here refers to the number of variables for base field elements
    // rather than SIMD field elements, the number of variables returned for calibration is also
    // over base field elements rather than SIMD field elements.
    pub fn from_random(
        world_size: usize,
        num_local_vars: usize,
        num_field_bits: usize,
        field_pack_size: usize,
        code_param_instance: OrionCodeParameter,
        mut rng: impl rand::RngCore,
    ) -> (Self, usize) {
        let (num_leaves_per_mt_query, scaled_num_local_vars, msg_size) =
            orion_eval_shape(world_size, num_local_vars, num_field_bits, field_pack_size);

        let srs_sampled = Self {
            num_vars: scaled_num_local_vars,
            num_leaves_per_mt_query,
            code_instance: OrionCode::new(code_param_instance, msg_size, &mut rng),
        };

        (srs_sampled, scaled_num_local_vars)
    }

    pub fn local_num_fs_per_query(&self) -> usize {
        let local_poly_len = 1 << self.num_vars;
        local_poly_len / self.message_len()
    }
}

pub type OrionCommitment = Node;

#[derive(Clone, Debug, Default, ExpSerde)]
pub struct OrionScratchPad {
    pub interleaved_alphabet_commitment: tree::Tree,
    pub merkle_cap: Vec<Node>,
}

#[derive(Clone, Debug, Default, ExpSerde)]
pub struct OrionProof<EvalF: Field> {
    pub eval_row: Vec<EvalF>,
    pub proximity_rows: Vec<Vec<EvalF>>,
    pub query_openings: Vec<tree::RangePath>,
    pub merkle_cap: Vec<Node>,
}

#[inline(always)]
pub(crate) fn commit_encoded<PackF>(
    pk: &OrionSRS,
    packed_evals: &[PackF],
    scratch_pad: &mut OrionScratchPad,
) -> OrionResult<OrionCommitment>
where
    PackF: SimdField,
{
    let packed_rows = pk.local_num_fs_per_query() / PackF::PACK_SIZE;

    // GPU commit disabled — testing GPU GKR + CPU PCS isolation
    #[cfg(feature = "cuda_pcs")]
    if false && packed_evals.len() >= 1048576 && std::env::var("USE_GPU_PROVER").is_ok() {
        let t0 = std::time::Instant::now();
        let commit_len = packed_evals.len();
        let msg_len = pk.message_len();
        let cw_len = pk.codeword_len();
        let srs_path = format!("/dev/shm/gpu_data/srs_{}.bin", commit_len);

        // Dump SRS if not yet done
        if !std::path::Path::new(&srs_path).exists() {
            std::fs::create_dir_all("/dev/shm/gpu_data").ok();
            use std::io::Write as W2;
            let code = &pk.code_instance;
            let mut f = std::fs::File::create(&srs_path).unwrap();
            f.write_all(&(code.msg_len() as u32).to_le_bytes()).unwrap();
            f.write_all(&(code.code_len() as u32).to_le_bytes()).unwrap();
            let n_graphs = code.g0s.len() + code.g1s.len();
            f.write_all(&(n_graphs as u32).to_le_bytes()).unwrap();
            f.write_all(&(pk.num_leaves_per_mt_query as u32).to_le_bytes()).unwrap();
            for gp in code.g0s.iter().chain(code.g1s.iter()) {
                f.write_all(&(gp.input_starts as u32).to_le_bytes()).unwrap();
                f.write_all(&(gp.output_starts as u32).to_le_bytes()).unwrap();
                f.write_all(&(gp.output_ends as u32).to_le_bytes()).unwrap();
                let g = &gp.graph;
                let mut row_ptrs = Vec::with_capacity(g.r_vertices_size + 1);
                row_ptrs.push(0u32);
                let mut col_indices = Vec::new();
                for ni in &g.neighborings {
                    for &idx in ni { col_indices.push(idx as u32); }
                    row_ptrs.push(col_indices.len() as u32);
                }
                for v in &row_ptrs { f.write_all(&v.to_le_bytes()).unwrap(); }
                for v in &col_indices { f.write_all(&v.to_le_bytes()).unwrap(); }
            }
        }

        {
            let srs_cstr = std::ffi::CString::new(srs_path.as_str()).unwrap();
            let total_m31x16 = (packed_rows * cw_len) as u64;
            let padded = total_m31x16.next_power_of_two() as usize;
            let mut root_hash = [0u8; 32];
            let mut n_leaves_out = 0u32;

            extern "C" {
                fn gpu_commit_to_tree(
                    packed_evals: *const u32, commit_len: u32, msg_len: u32, cw_len: u32,
                    srs_path: *const std::ffi::c_char,
                    root_hash: *mut u8, n_leaves: *mut u32) -> i32;
                fn gpu_tree_download(
                    tree_id: i32,
                    leaf_hashes: *mut u8, nodes: *mut u8, leaves_raw: *mut u8);
                fn gpu_tree_free(tree_id: i32);
            }

            // GPU commit — tree stays in VRAM, only root hash downloaded (~32 bytes)
            let tree_id = unsafe {
                gpu_commit_to_tree(
                    packed_evals.as_ptr() as *const u32,
                    commit_len as u32, msg_len as u32, cw_len as u32,
                    srs_cstr.as_ptr(),
                    root_hash.as_mut_ptr(), &mut n_leaves_out)
            };
            let n_leaves = n_leaves_out as usize;
            let t_commit_done = t0.elapsed();

            // Download tree from GPU for PCS compatibility
            // TODO: keep tree GPU-resident, implement gpu_merkle_query for on-demand extraction
            let mut leaf_hashes_raw = vec![0u8; n_leaves * 32];
            let mut nodes_raw = vec![0u8; (n_leaves - 1) * 32];
            let mut leaves_raw = vec![0u8; n_leaves * 64];
            if tree_id >= 0 {
                unsafe {
                    gpu_tree_download(tree_id,
                        leaf_hashes_raw.as_mut_ptr(), nodes_raw.as_mut_ptr(),
                        leaves_raw.as_mut_ptr());
                    gpu_tree_free(tree_id);
                }
            }

            // Reconstruct Tree
            let leaves: Vec<tree::Leaf> = unsafe {
                let ptr = leaves_raw.as_ptr() as *const tree::Leaf;
                std::slice::from_raw_parts(ptr, n_leaves).to_vec()
            };
            let mut nodes: Vec<tree::Node> = unsafe {
                let ptr = nodes_raw.as_ptr() as *const tree::Node;
                std::slice::from_raw_parts(ptr, n_leaves - 1).to_vec()
            };
            let leaf_nodes: Vec<tree::Node> = unsafe {
                let ptr = leaf_hashes_raw.as_ptr() as *const tree::Node;
                std::slice::from_raw_parts(ptr, n_leaves).to_vec()
            };
            nodes.extend(leaf_nodes);

            scratch_pad.interleaved_alphabet_commitment = tree::Tree { nodes, leaves };
            scratch_pad.merkle_cap = vec![scratch_pad.interleaved_alphabet_commitment.root()];
            eprintln!("    [gpu-commit] {} leaves: compute={:?} total={:?}", n_leaves, t_commit_done, t0.elapsed());
            return Ok(scratch_pad.interleaved_alphabet_commitment.root());
        } // end inner block
    }

    let t_commit = std::time::Instant::now();

    // NOTE: packed codeword buffer and encode over packed field (parallel)
    let mut codewords = vec![PackF::ZERO; packed_rows * pk.codeword_len()];
    {
        use rayon::prelude::*;
        let code = &pk.code_instance;
        let msg_len = pk.message_len();
        let cw_len = pk.codeword_len();
        packed_evals.par_chunks(msg_len)
            .zip(codewords.par_chunks_mut(cw_len))
            .for_each(|(evals, codeword)| {
                code.encode_in_place(evals, codeword).unwrap();
            });
    }

    let t_encode_done = std::time::Instant::now();
    // NOTE: transpose codeword s.t., the matrix has codewords being columns
    let mut scratch = vec![PackF::ZERO; std::cmp::max(packed_rows, pk.codeword_len())];
    transpose_inplace(&mut codewords, &mut scratch, pk.codeword_len(), packed_rows);
    drop(scratch);
    let t_transpose_done = std::time::Instant::now();
    // NOTE: commit the interleaved codeword
    // we just directly commit to the packed field elements to leaves
    // Also note, when codeword is not power of 2 length, pad to nearest po2
    // to commit by merkle tree
    if !codewords.len().is_power_of_two() {
        let aligned_po2_len = codewords.len().next_power_of_two();
        codewords.resize(aligned_po2_len, PackF::ZERO);
    }
    scratch_pad.interleaved_alphabet_commitment =
        tree::Tree::compact_new_with_packed_field_elems(codewords);

    scratch_pad.merkle_cap = vec![scratch_pad.interleaved_alphabet_commitment.root()];

    let t_tree_done = std::time::Instant::now();
    eprintln!("    [commit-inner] encode={:?} transpose={:?} tree={:?} rows={} cw_len={}",
        t_encode_done - t_commit, t_transpose_done - t_encode_done,
        t_tree_done - t_transpose_done, packed_rows, pk.codeword_len());
    Ok(scratch_pad.interleaved_alphabet_commitment.root())
}

#[inline(always)]
pub(crate) fn orion_mt_openings<T>(
    pk: &OrionSRS,
    transcript: &mut T,
    scratch_pad: &OrionScratchPad,
) -> Vec<tree::RangePath>
where
    T: Transcript,
{
    let leaves_in_range_opening = pk.num_leaves_per_mt_query();

    // NOTE: MT opening for point queries
    let query_num = pk.query_complexity(PCS_SOUNDNESS_BITS);
    let query_indices = transcript.generate_usize_vector(query_num);
    query_indices
        .iter()
        .map(|qi| {
            let index = *qi % pk.codeword_len();
            let left = index * leaves_in_range_opening;
            let right = left + leaves_in_range_opening - 1;

            scratch_pad
                .interleaved_alphabet_commitment
                .range_query(left, right)
        })
        .collect()
}

#[inline(always)]
pub(crate) fn orion_mt_verify(
    vk: &OrionSRS,
    query_indices: &[usize],
    range_openings: &[tree::RangePath],
    merkle_cap: &[Node],
) -> bool {
    let world_size = merkle_cap.len();
    let indices_per_merkle_cap = vk.codeword_len().next_power_of_two() / world_size;

    izip!(query_indices, range_openings).all(|(&qi, range_path)| {
        let index = qi % vk.codeword_len();
        let merkle_cap_index = index / indices_per_merkle_cap;
        let in_sub_tree_index = index % indices_per_merkle_cap;

        range_path.verify(&merkle_cap[merkle_cap_index])
            && in_sub_tree_index == range_path.left / range_path.leaves.len()
    })
}

/*
 * LINEAR OPERATIONS FOR GF2 (LOOKUP TABLE BASED)
 */

pub struct SubsetSumLUTs<F: Field> {
    pub entry_bits: usize,
    pub tables: Vec<Vec<F>>,
}

impl<F: Field> SubsetSumLUTs<F> {
    #[inline(always)]
    pub fn new(entry_bits: usize, table_num: usize) -> Self {
        assert!(entry_bits > 0 && table_num > 0);

        Self {
            entry_bits,
            tables: vec![vec![F::ZERO; 1 << entry_bits]; table_num],
        }
    }

    #[inline(always)]
    pub fn build(&mut self, weights: &[F]) {
        assert_eq!(weights.len(), self.entry_bits * self.tables.len());

        self.tables.iter_mut().for_each(|lut| lut.fill(F::ZERO));

        // NOTE: we are assuming that the table is for {0, 1}-linear combination
        izip!(&mut self.tables, weights.chunks(self.entry_bits)).for_each(
            |(lut_i, sub_weights)| {
                sub_weights.iter().enumerate().for_each(|(i, weight_i)| {
                    let bit_mask = 1 << i;
                    lut_i.iter_mut().enumerate().for_each(|(bit_map, li)| {
                        if bit_map & bit_mask == bit_mask {
                            *li += weight_i;
                        }
                    });
                });
            },
        );
    }

    #[inline(always)]
    pub fn lookup_and_sum<EntryF>(&self, indices: &[EntryF]) -> F
    where
        EntryF: SimdField,
    {
        // NOTE: at least the entry field elem should have a matching field size
        // and the the entry field being a SIMD field with same packing size as
        // the bits for the table entry
        assert_eq!(EntryF::FIELD_SIZE, 1);
        assert_eq!(EntryF::PACK_SIZE, self.entry_bits);
        assert_eq!(indices.len(), self.tables.len());

        izip!(&self.tables, indices)
            .map(|(t_i, index)| t_i[index.as_u32_unchecked() as usize])
            .sum()
    }
}

#[inline(always)]
pub(crate) fn lut_open_linear_combine<F, EvalF, SimdF>(
    com_pack_size: usize,
    packed_evals: &[SimdF],
    eq_col_coeffs: &[EvalF],
    eval_row: &mut [EvalF],
    rand_col_coeffs: &[Vec<EvalF>],
    proximity_rows: &mut [Vec<EvalF>],
) where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    SimdF: SimdField<Scalar = F>,
{
    // NOTE: declare the look up tables for column sums
    let table_num = com_pack_size / SimdF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::<EvalF>::new(SimdF::PACK_SIZE, table_num);
    assert_eq!(com_pack_size % SimdF::PACK_SIZE, 0);

    let combination_size = eq_col_coeffs.len();
    let packed_row_size = combination_size / com_pack_size;

    // NOTE: working on evaluation response of tensor code IOP based PCS
    izip!(
        eq_col_coeffs.chunks(com_pack_size),
        packed_evals.chunks(packed_evals.len() / packed_row_size)
    )
    .for_each(|(eq_chunk, packed_evals_chunk)| {
        luts.build(eq_chunk);

        izip!(packed_evals_chunk.chunks(table_num), &mut *eval_row)
            .for_each(|(p_col, eval)| *eval += luts.lookup_and_sum(p_col));
    });

    // NOTE: compose proximity response(s) of tensor code IOP based PCS
    izip!(proximity_rows, rand_col_coeffs).for_each(|(row_buffer, random_coeffs)| {
        izip!(
            random_coeffs.chunks(com_pack_size),
            packed_evals.chunks(packed_evals.len() / packed_row_size)
        )
        .for_each(|(random_chunk, packed_evals_chunk)| {
            luts.build(random_chunk);

            izip!(packed_evals_chunk.chunks(table_num), &mut *row_buffer)
                .for_each(|(p_col, prox)| *prox += luts.lookup_and_sum(p_col));
        });
    });
    drop(luts);
}

#[inline(always)]
pub(crate) fn lut_verify_alphabet_check<F, SimdF, ExtF>(
    codeword: &[ExtF],
    fixed_rl: &[ExtF],
    query_indices: &[usize],
    packed_interleaved_alphabets: &[Vec<SimdF>],
) -> bool
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ExtF: ExtensionField<BaseField = F>,
{
    // NOTE: build up lookup table
    let tables_num = fixed_rl.len() / SimdF::PACK_SIZE;
    assert_eq!(fixed_rl.len() % SimdF::PACK_SIZE, 0);
    let mut luts = SubsetSumLUTs::<ExtF>::new(SimdF::PACK_SIZE, tables_num);

    luts.build(fixed_rl);

    izip!(query_indices, packed_interleaved_alphabets).all(|(qi, interleaved_alphabet)| {
        let index = qi % codeword.len();
        let alphabet = luts.lookup_and_sum(interleaved_alphabet);
        alphabet == codeword[index]
    })
}

/*
 * LINEAR OPERATIONS FOR MERSENNE31 (SIMD BASED)
 */

#[inline(always)]
fn simd_ext_base_inner_prod<F, ExtF, SimdF>(
    simd_ext_limbs: &[SimdF],
    simd_base_elems: &[SimdF],
) -> ExtF
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ExtF: ExtensionField<BaseField = F>,
{
    assert_eq!(simd_ext_limbs.len(), simd_base_elems.len() * ExtF::DEGREE);

    let mut ext_limbs = vec![F::ZERO; ExtF::DEGREE];

    izip!(&mut ext_limbs, simd_ext_limbs.chunks(simd_base_elems.len())).for_each(
        |(e, simd_ext_limb)| {
            let simd_sum: SimdF = izip!(simd_ext_limb, simd_base_elems)
                .map(|(a, b)| *a * b)
                .sum();
            *e = simd_sum.horizontal_sum();
        },
    );

    ExtF::from_limbs(&ext_limbs)
}

#[inline(always)]
fn gpu_linear_combine<F, EvalF, SimdF>(
    packed_evals: &[SimdF],
    eq_col_coeffs: &[EvalF],
    eval_row: &mut [EvalF],
    rand_col_coeffs: &[Vec<EvalF>],
    proximity_rows: &mut [Vec<EvalF>],
    com_pack_size: usize,
    simd_inner_prods: usize,
    packed_row_size: usize,
) where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    SimdF: SimdField<Scalar = F>,
{
    use std::io::{BufRead, BufReader, Write};
    let msg_len = eval_row.len();
    let packed_rows = eq_col_coeffs.len() / com_pack_size;
    let commit_len = packed_evals.len();
    let n_prox = proximity_rows.len();

    // Write request to /dev/shm/gpu_data/lc_request.bin
    let req_path = "/dev/shm/gpu_data/lc_request.bin";
    {
        let mut f = std::fs::File::create(req_path).unwrap();
        f.write_all(&1u32.to_le_bytes()).unwrap(); // 1 job
        f.write_all(&(commit_len as u32).to_le_bytes()).unwrap();
        f.write_all(&(msg_len as u32).to_le_bytes()).unwrap();
        f.write_all(&(packed_rows as u32).to_le_bytes()).unwrap();
        f.write_all(&(n_prox as u32).to_le_bytes()).unwrap();

        // packed_evals as raw bytes
        let evals_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(packed_evals.as_ptr() as *const u8,
                                        packed_evals.len() * std::mem::size_of::<SimdF>())
        };
        f.write_all(evals_bytes).unwrap();

        // eq_col_coeffs as raw E3 bytes (packed_rows * 16 * 12 bytes)
        let eq_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(eq_col_coeffs.as_ptr() as *const u8,
                                        eq_col_coeffs.len() * std::mem::size_of::<EvalF>())
        };
        f.write_all(eq_bytes).unwrap();

        // rand_col_coeffs per proximity test
        for rc in rand_col_coeffs {
            let rc_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(rc.as_ptr() as *const u8,
                                            rc.len() * std::mem::size_of::<EvalF>())
            };
            f.write_all(rc_bytes).unwrap();
        }
    }

    // Send LINEAR_COMBINE to GPU server
    gpu_server_command("LINEAR_COMBINE");

    // Read results
    let res_path = "/dev/shm/gpu_data/lc_result.bin";
    let res_data = std::fs::read(res_path).unwrap();
    let elem_size = std::mem::size_of::<EvalF>();
    let mut offset = 0;

    // eval_row
    unsafe {
        let src = &res_data[offset..offset + msg_len * elem_size];
        std::ptr::copy_nonoverlapping(src.as_ptr(), eval_row.as_mut_ptr() as *mut u8, src.len());
    }
    offset += msg_len * elem_size;

    // proximity_rows
    for pr in proximity_rows.iter_mut() {
        unsafe {
            let src = &res_data[offset..offset + msg_len * elem_size];
            std::ptr::copy_nonoverlapping(src.as_ptr(), pr.as_mut_ptr() as *mut u8, src.len());
        }
        offset += msg_len * elem_size;
    }
}

fn gpu_server_command(cmd: &str) {
    use std::io::{BufRead, BufReader, Write};
    use std::sync::Mutex;

    static GPU_SERVER: std::sync::OnceLock<Mutex<GpuServerConn>> = std::sync::OnceLock::new();

    struct GpuServerConn {
        stdin: std::process::ChildStdin,
        stdout: BufReader<std::process::ChildStdout>,
    }

    let server = GPU_SERVER.get_or_init(|| {
        let mut child = std::process::Command::new("cuda/gpu_prover_v2")
            .arg("--server")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .expect("Failed to launch cuda/gpu_prover_v2 --server");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut s = GpuServerConn { stdin, stdout };
        let mut line = String::new();
        s.stdout.read_line(&mut line).unwrap();
        assert!(line.starts_with("GPU_SERVER_READY"), "GPU: {}", line.trim());
        eprintln!("  [gpu-lc] server connected");
        Mutex::new(s)
    });

    let mut s = server.lock().unwrap();
    writeln!(s.stdin, "{}", cmd).unwrap();
    s.stdin.flush().unwrap();
    let mut line = String::new();
    s.stdout.read_line(&mut line).unwrap();
    eprintln!("  [gpu-lc] {}: {}", cmd, line.trim());
}

pub(crate) fn simd_open_linear_combine<F, EvalF, SimdF>(
    com_pack_size: usize,
    packed_evals: &[SimdF],
    eq_col_coeffs: &[EvalF],
    eval_row: &mut [EvalF],
    rand_col_coeffs: &[Vec<EvalF>],
    proximity_rows: &mut [Vec<EvalF>],
) where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    SimdF: SimdField<Scalar = F>,
{
    // NOTE: check SIMD inner product numbers for column sums
    let simd_inner_prods = com_pack_size / SimdF::PACK_SIZE;
    assert_eq!(com_pack_size % SimdF::PACK_SIZE, 0);

    let combination_size = eq_col_coeffs.len();
    let packed_row_size = combination_size / com_pack_size;

    let mut buffer = vec![F::ZERO; com_pack_size * EvalF::DEGREE];

    // GPU FFI acceleration for large commits
    #[cfg(feature = "cuda_pcs")]
    if eval_row.len() >= 32768 {
        let t_gpu = std::time::Instant::now();
        let msg_len = eval_row.len();
        let packed_rows = eq_col_coeffs.len() / com_pack_size;
        let n_prox = proximity_rows.len();

        // Flatten eq_col_coeffs to raw M31 (3 per E3)
        let eq_flat: Vec<u32> = unsafe {
            std::slice::from_raw_parts(eq_col_coeffs.as_ptr() as *const u32,
                eq_col_coeffs.len() * 3).to_vec()
        };
        // Flatten rand_col_coeffs
        let mut rand_flat: Vec<u32> = Vec::with_capacity(n_prox * packed_rows * 16 * 3);
        for rc in rand_col_coeffs {
            unsafe {
                rand_flat.extend_from_slice(std::slice::from_raw_parts(
                    rc.as_ptr() as *const u32, rc.len() * 3));
            }
        }
        // packed_evals as raw u32
        let evals_ptr = packed_evals.as_ptr() as *const u32;
        let eval_row_ptr = eval_row.as_mut_ptr() as *mut u32;

        // Flatten prox output
        let mut prox_flat = vec![0u32; n_prox * msg_len * 3];

        extern "C" {
            fn gpu_linear_combine_m31ext3(
                packed_evals: *const u32, eq_coeffs: *const u32,
                packed_rows: u32, msg_len: u32, eval_row: *mut u32,
                n_proximity: u32, rand_coeffs: *const u32, prox_rows: *mut u32);
        }
        unsafe {
            gpu_linear_combine_m31ext3(
                evals_ptr, eq_flat.as_ptr(), packed_rows as u32, msg_len as u32,
                eval_row_ptr, n_prox as u32, rand_flat.as_ptr(), prox_flat.as_mut_ptr());
        }

        // Copy prox results back
        for (t, pr) in proximity_rows.iter_mut().enumerate() {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    prox_flat[t * msg_len * 3..].as_ptr(),
                    pr.as_mut_ptr() as *mut u32, msg_len * 3);
            }
        }
        eprintln!("      [gpu-ffi] linear_combine: {:?}", t_gpu.elapsed());
        return;
    }

    // eval_row: parallel over j within each chunk pair
    {
        use rayon::prelude::*;
        let msg_len = eval_row.len();
        // Pre-compute SIMD limbs per chunk (avoids redundant work in parallel loop)
        let chunk_limbs: Vec<Vec<SimdF>> = eq_col_coeffs.chunks(com_pack_size).map(|eq_chunk| {
            let mut buf = vec![F::ZERO; com_pack_size * EvalF::DEGREE];
            let eq_limbs: Vec<_> = eq_chunk.iter().flat_map(|e| e.to_limbs()).collect();
            transpose(&eq_limbs, &mut buf, EvalF::DEGREE, com_pack_size);
            buf.chunks(SimdF::PACK_SIZE).map(SimdF::pack).collect()
        }).collect();

        let data_chunk_size = packed_evals.len() / packed_row_size;
        let actual_msg = data_chunk_size / simd_inner_prods;
        for (r, simd_limbs) in chunk_limbs.iter().enumerate() {
            let data_chunk = &packed_evals[r * data_chunk_size..(r + 1) * data_chunk_size];
            if actual_msg >= 256 {
                eval_row[..actual_msg].par_iter_mut().enumerate().for_each(|(j, eval)| {
                    let p_col = &data_chunk[j * simd_inner_prods..(j + 1) * simd_inner_prods];
                    *eval += simd_ext_base_inner_prod::<_, EvalF, _>(simd_limbs, p_col);
                });
            } else {
                izip!(data_chunk.chunks(simd_inner_prods), &mut eval_row[..actual_msg]).for_each(
                    |(p_col, eval)| *eval += simd_ext_base_inner_prod::<_, EvalF, _>(simd_limbs, p_col),
                );
            }
        }
    }

    // proximity_rows: parallel across tests (each test is independent)
    {
        use rayon::prelude::*;
        proximity_rows.par_iter_mut().zip(rand_col_coeffs.par_iter()).for_each(|(prox_row, random_coeffs)| {
            let mut buf = vec![F::ZERO; com_pack_size * EvalF::DEGREE];
            izip!(
                random_coeffs.chunks(com_pack_size),
                packed_evals.chunks(packed_evals.len() / packed_row_size)
            )
            .for_each(|(rand_chunk, packed_evals_chunk)| {
                let rand_limbs: Vec<_> = rand_chunk.iter().flat_map(|e| e.to_limbs()).collect();
                transpose(&rand_limbs, &mut buf, EvalF::DEGREE, com_pack_size);
                let simd_limbs: Vec<_> = buf.chunks(SimdF::PACK_SIZE).map(SimdF::pack).collect();
                izip!(packed_evals_chunk.chunks(simd_inner_prods), &mut *prox_row).for_each(
                    |(p_col, prox)| {
                        *prox += simd_ext_base_inner_prod::<_, EvalF, _>(&simd_limbs, p_col)
                    },
                );
            });
        });
    }
}

#[inline(always)]
pub(crate) fn simd_verify_alphabet_check<F, SimdF, ExtF>(
    codeword: &[ExtF],
    fixed_rl: &[ExtF],
    query_indices: &[usize],
    packed_interleaved_alphabets: &[Vec<SimdF>],
) -> bool
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ExtF: ExtensionField<BaseField = F>,
{
    // NOTE: check SIMD inner product numbers for column sums
    assert_eq!(fixed_rl.len() % SimdF::PACK_SIZE, 0);

    let mut scratch = vec![F::ZERO; fixed_rl.len() * ExtF::DEGREE];
    let rl_limbs: Vec<_> = fixed_rl.iter().flat_map(|e| e.to_limbs()).collect();
    transpose(&rl_limbs, &mut scratch, ExtF::DEGREE, fixed_rl.len());
    let simd_limbs: Vec<_> = scratch.chunks(SimdF::PACK_SIZE).map(SimdF::pack).collect();

    izip!(query_indices, packed_interleaved_alphabets).all(|(qi, interleaved_alphabet)| {
        let index = qi % codeword.len();
        let alphabet: ExtF = simd_ext_base_inner_prod(&simd_limbs, interleaved_alphabet);
        alphabet == codeword[index]
    })
}

#[cfg(test)]
mod tests {
    use arith::{Field, SimdField};
    use ark_std::test_rng;
    use gf2::{GF2x8, GF2};
    use gf2_128::{GF2_128x8, GF2_128};
    use itertools::izip;

    use crate::orion::utils::SubsetSumLUTs;

    #[test]
    fn test_lut_simd_inner_prod_consistency() {
        let mut rng = test_rng();

        let weights: Vec<_> = (0..8).map(|_| GF2_128::random_unsafe(&mut rng)).collect();
        let bases: Vec<_> = (0..8).map(|_| GF2::random_unsafe(&mut rng)).collect();

        let simd_weights = GF2_128x8::pack(&weights);
        let simd_bases = GF2x8::pack(&bases);

        let expected_simd_inner_prod: GF2_128 = (simd_weights * simd_bases).horizontal_sum();

        let expected_vanilla_inner_prod: GF2_128 =
            izip!(&weights, &bases).map(|(w, b)| *w * *b).sum();

        assert_eq!(expected_simd_inner_prod, expected_vanilla_inner_prod);

        let mut table = SubsetSumLUTs::new(8, 1);
        table.build(&weights);

        let actual_lut_inner_prod = table.lookup_and_sum(&[simd_bases]);

        assert_eq!(expected_simd_inner_prod, actual_lut_inner_prod)
    }
}
