use arith::{ExtensionField, Field, SimdField};
use gf2::GF2;
use gkr_engine::Transcript;
use polynomials::{EqPolynomial, MultilinearExtension, RefMultiLinearPoly};

use crate::{
    orion::{
        utils::{
            commit_encoded, lut_open_linear_combine, orion_mt_openings, simd_open_linear_combine,
        },
        OrionCommitment, OrionProof, OrionResult, OrionSRS, OrionScratchPad,
    },
    traits::TensorCodeIOPPCS,
    PCS_SOUNDNESS_BITS,
};

#[inline(always)]
pub fn orion_commit_simd_field<F, SimdF, ComPackF>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<SimdF>,
    scratch_pad: &mut OrionScratchPad,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let packed_evals_ref = unsafe {
        let relative_pack_size = ComPackF::PACK_SIZE / SimdF::PACK_SIZE;
        assert_eq!(ComPackF::PACK_SIZE % SimdF::PACK_SIZE, 0);

        let ptr = poly.hypercube_basis_ref().as_ptr();
        let len = poly.hypercube_size() / relative_pack_size;
        assert_eq!(len * relative_pack_size, poly.hypercube_size());

        std::slice::from_raw_parts(ptr as *const ComPackF, len)
    };

    commit_encoded(pk, packed_evals_ref, scratch_pad)
}

#[inline(always)]
pub fn orion_open_simd_field<F, SimdF, EvalF, ComPackF>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<SimdF>,
    point: &[EvalF],
    transcript: &mut impl Transcript,
    scratch_pad: &OrionScratchPad,
) -> (EvalF, OrionProof<EvalF>)
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let msg_size = pk.message_len();

    let num_vars_in_com_simd = ComPackF::PACK_SIZE.ilog2() as usize;
    let num_vars_in_msg = msg_size.ilog2() as usize;

    // NOTE: pre-compute the eq linear combine coeffs for linear combination
    let eq_col_coeffs = {
        let mut eq_vars = point[..num_vars_in_com_simd].to_vec();
        eq_vars.extend_from_slice(&point[num_vars_in_com_simd + num_vars_in_msg..]);
        EqPolynomial::build_eq_x_r(&eq_vars)
    };

    // NOTE: pre-declare the spaces for returning evaluation and proximity queries
    let mut eval_row = vec![EvalF::ZERO; msg_size];

    let proximity_test_num = pk.proximity_repetitions::<EvalF>(PCS_SOUNDNESS_BITS);
    let mut proximity_rows = vec![vec![EvalF::ZERO; msg_size]; proximity_test_num];

    let random_col_coeffs: Vec<_> = (0..proximity_test_num)
        .map(|_| {
            let rand = transcript.generate_field_elements::<EvalF>(point.len() - num_vars_in_msg);
            EqPolynomial::build_eq_x_r(&rand)
        })
        .collect();

    // Check if tree is GPU-resident
    let root_bytes: [u8; 32] = unsafe {
        std::ptr::read(scratch_pad.merkle_cap.get(0).map(|n| n as *const _ as *const [u8; 32]).unwrap_or(&std::ptr::null::<[u8;32]>() as *const _ as *const [u8;32]))
    };
    let gpu_tree = super::utils::GPU_TREE_REGISTRY.lock().unwrap().get(&root_bytes).copied();

    if let Some((tree_id, _gpu_n_leaves)) = gpu_tree {
        // GPU PCS open: linear_combine + proximity + Merkle paths all on GPU
        let _t_gpu = std::time::Instant::now();
        let packed_rows = poly.hypercube_basis_ref().len() / msg_size;
        let commit_len = packed_rows * msg_size;

        // Flatten coefficients for FFI
        let eq_flat: Vec<u32> = unsafe {
            std::slice::from_raw_parts(eq_col_coeffs.as_ptr() as *const u32,
                eq_col_coeffs.len() * 3).to_vec()
        };
        let rand_flat: Vec<u32> = unsafe {
            let total = random_col_coeffs.iter().map(|v| v.len()).sum::<usize>();
            let mut buf = Vec::with_capacity(total * 3);
            for rc in &random_col_coeffs {
                buf.extend_from_slice(std::slice::from_raw_parts(rc.as_ptr() as *const u32, rc.len() * 3));
            }
            buf
        };

        // Query indices from transcript
        let query_num = pk.query_complexity(PCS_SOUNDNESS_BITS);
        let query_indices_raw = transcript.generate_usize_vector(query_num);
        let leaves_per_query = pk.num_leaves_per_mt_query();
        let cw_len = pk.codeword_len();
        let query_positions: Vec<u32> = query_indices_raw.iter()
            .map(|qi| (*qi % cw_len) as u32).collect();

        // Output buffers
        let mut h_eval_row = vec![0u32; msg_size * 3];
        let mut h_prox_rows = vec![0u32; proximity_test_num * msg_size * 3];
        let mut h_query_leaves = vec![0u8; query_num * leaves_per_query * 64];
        let mut h_query_siblings = vec![0u8; query_num * 32 * 32]; // max 32 levels
        let mut tree_depth = 0u32;

        extern "C" {
            fn gpu_pcs_open_with_device_data(
                d_packed_evals: *const u32, h_eq_coeffs: *const u32,
                packed_rows: u32, msg_len: u32, h_eval_row: *mut u32,
                n_proximity: u32, h_rand_coeffs: *const u32, h_prox_rows: *mut u32,
                tree_id: i32, h_query_indices: *const u32, n_queries: u32,
                leaves_per_query: u32, h_query_leaves: *mut u8,
                h_query_path_nodes: *mut u8, out_tree_depth: *mut u32);
        }

        // Get device polynomial pointer from tree slot
        extern "C" {
            fn gpu_tree_get_ptrs(id: i32, leaves: *mut *mut u32, lh: *mut *mut u8,
                nd: *mut *mut u8, n: *mut u32, poly: *mut *mut u32,
                cl: *mut u32, ml: *mut u32);
        }
        let mut d_poly: *mut u32 = std::ptr::null_mut();
        let mut _cl = 0u32; let mut _ml = 0u32;
        let mut _lv: *mut u32 = std::ptr::null_mut();
        let mut _lh: *mut u8 = std::ptr::null_mut();
        let mut _nd: *mut u8 = std::ptr::null_mut();
        let mut _nl = 0u32;
        unsafe {
            gpu_tree_get_ptrs(tree_id, &mut _lv, &mut _lh, &mut _nd, &mut _nl,
                &mut d_poly, &mut _cl, &mut _ml);
        }

        unsafe {
            gpu_pcs_open_with_device_data(
                d_poly as *const u32, eq_flat.as_ptr(),
                packed_rows as u32, msg_size as u32, h_eval_row.as_mut_ptr(),
                proximity_test_num as u32, rand_flat.as_ptr(), h_prox_rows.as_mut_ptr(),
                tree_id, query_positions.as_ptr(), query_num as u32,
                leaves_per_query as u32, h_query_leaves.as_mut_ptr(),
                h_query_siblings.as_mut_ptr(), &mut tree_depth);
        }

        // Convert results back to Rust types
        let eval_row: Vec<EvalF> = unsafe {
            std::slice::from_raw_parts(h_eval_row.as_ptr() as *const EvalF, msg_size).to_vec()
        };
        let proximity_rows: Vec<Vec<EvalF>> = (0..proximity_test_num)
            .map(|t| unsafe {
                std::slice::from_raw_parts(
                    (h_prox_rows.as_ptr() as *const EvalF).add(t * msg_size), msg_size).to_vec()
            }).collect();

        // Compute eval from eval_row
        let mut scratch = vec![EvalF::ZERO; msg_size];
        let eval = RefMultiLinearPoly::from_ref(&eval_row).evaluate_with_buffer(
            &point[num_vars_in_com_simd..num_vars_in_com_simd + num_vars_in_msg],
            &mut scratch,
        );

        // Reconstruct RangePaths from GPU query results
        let depth = tree_depth as usize;
        let query_openings: Vec<tree::RangePath> = (0..query_num).map(|qi| {
            let pos = query_positions[qi] as usize;
            let left = pos * leaves_per_query;
            let right = left + leaves_per_query - 1;
            let leaf_data = &h_query_leaves[qi * leaves_per_query * 64..(qi + 1) * leaves_per_query * 64];
            let leaves: Vec<tree::Leaf> = unsafe {
                std::slice::from_raw_parts(leaf_data.as_ptr() as *const tree::Leaf, leaves_per_query).to_vec()
            };
            let sibling_data = &h_query_siblings[qi * depth * 32..(qi + 1) * depth * 32];
            let path_nodes: Vec<tree::Node> = unsafe {
                std::slice::from_raw_parts(sibling_data.as_ptr() as *const tree::Node, depth).to_vec()
            };
            tree::RangePath { leaves, path_nodes, left, right }
        }).collect();

        eprintln!("      [gpu-pcs-open] {:?} (lc+prox+merkle all GPU)", _t_gpu.elapsed());
        (
            eval,
            OrionProof {
                eval_row,
                proximity_rows,
                query_openings,
                merkle_cap: scratch_pad.merkle_cap.clone(),
            },
        )
    } else {
        // CPU path (original)
        let _t_lc = std::time::Instant::now();
        match F::NAME {
            GF2::NAME => lut_open_linear_combine(
                ComPackF::PACK_SIZE,
                poly.hypercube_basis_ref(),
                &eq_col_coeffs,
                &mut eval_row,
                &random_col_coeffs,
                &mut proximity_rows,
            ),
            _ => simd_open_linear_combine(
                ComPackF::PACK_SIZE,
                poly.hypercube_basis_ref(),
                &eq_col_coeffs,
                &mut eval_row,
                &random_col_coeffs,
                &mut proximity_rows,
            ),
        }

        eprintln!("      [pcs-inner] linear_combine: {:?}", _t_lc.elapsed());
        let _t_eval = std::time::Instant::now();
        let mut scratch = vec![EvalF::ZERO; msg_size];
        let eval = RefMultiLinearPoly::from_ref(&eval_row).evaluate_with_buffer(
            &point[num_vars_in_com_simd..num_vars_in_com_simd + num_vars_in_msg],
            &mut scratch,
        );
        drop(scratch);

        eprintln!("      [pcs-inner] multilinear_eval: {:?}", _t_eval.elapsed());
        let _t_mt = std::time::Instant::now();
        let query_openings = orion_mt_openings(pk, transcript, scratch_pad);

        eprintln!("      [pcs-inner] merkle_query: {:?} ({} queries)", _t_mt.elapsed(), query_openings.len());
        (
            eval,
            OrionProof {
                eval_row,
                proximity_rows,
                query_openings,
                merkle_cap: scratch_pad.merkle_cap.clone(),
            },
        )
    }
}
