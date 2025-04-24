use arith::{FFTField, Field};
use tree::{Leaf, LEAF_BYTES};

pub type FRICommitment = tree::Node;

#[derive(Clone, Debug, Default)]
pub struct FRIScratchPad<F: FFTField> {
    pub merkle: tree::Tree,
    pub codeword: Vec<F>,
    pub rate_log2: usize,
}

unsafe impl<F: FFTField> Send for FRIScratchPad<F> {}

#[derive(Clone, Debug, Default)]
pub struct FRIOpening<F: Field> {
    pub iopp_oracles: Vec<tree::Node>,
    pub iopp_queries: Vec<Vec<(tree::Path, tree::Path)>>,
    pub sumcheck_responses: Vec<Vec<F>>,
}

#[inline(always)]
pub(crate) fn copy_elems_to_leaves<F: Field>(elems: &[F]) -> Vec<Leaf> {
    let max_elems_per_leaf = LEAF_BYTES * 8 / F::FIELD_SIZE;
    let num_elems_per_leaf = if max_elems_per_leaf.is_power_of_two() {
        max_elems_per_leaf
    } else {
        max_elems_per_leaf.next_power_of_two() / 2
    };

    assert!(num_elems_per_leaf * F::FIELD_SIZE <= LEAF_BYTES * 8);
    assert_eq!(elems.len() % num_elems_per_leaf, 0);

    let field_bytes = F::FIELD_SIZE / 8;

    elems
        .chunks(num_elems_per_leaf)
        .map(|elems_chunk| {
            let mut leaf = Leaf::default();

            elems_chunk.iter().enumerate().for_each(|(i, e)| {
                let begin = i * field_bytes;
                let end = begin + field_bytes;

                e.serialize_into(&mut leaf.data[begin..end]).unwrap()
            });

            leaf
        })
        .collect()
}

#[inline(always)]
pub(crate) fn fri_mt_opening(
    point_to_alphabet: &mut usize,
    codeword_len: usize,
    merkle_tree: &tree::Tree,
) -> (tree::Path, tree::Path) {
    let elems_in_leaf = codeword_len / merkle_tree.size();
    let point_to_leaf = *point_to_alphabet / elems_in_leaf;

    let oracle_rhs_start = merkle_tree.size() >> 1;
    let sibling_point = point_to_leaf ^ oracle_rhs_start;

    let left = std::cmp::min(point_to_leaf, sibling_point);
    let right = oracle_rhs_start + left;

    let height = merkle_tree.height();

    if *point_to_alphabet >= codeword_len / 2 {
        *point_to_alphabet -= codeword_len / 2
    }

    (
        merkle_tree.gen_proof(left, height),
        merkle_tree.gen_proof(right, height),
    )
}

#[inline(always)]
pub(crate) fn fri_mt_query_alphabet<F: Field>(
    query: &tree::Path,
    alphabet_index_in_leaf: usize,
) -> F {
    let field_bytes = F::FIELD_SIZE / 8;

    let begin = alphabet_index_in_leaf * field_bytes;
    let end = begin + field_bytes;

    let mut buffer = vec![0u8; field_bytes];
    buffer.copy_from_slice(&query.leaf.data[begin..end]);

    F::deserialize_from(buffer.as_slice()).unwrap()
}

#[inline(always)]
pub(crate) fn fri_alphabets<F: Field>(
    point_to_alphabet: &mut usize,
    codeword_len: usize,
    query_pair: &(tree::Path, tree::Path),
) -> (F, F) {
    let max_elems_per_leaf = LEAF_BYTES * 8 / F::FIELD_SIZE;
    let num_elems_per_leaf = if max_elems_per_leaf.is_power_of_two() {
        max_elems_per_leaf
    } else {
        max_elems_per_leaf.next_power_of_two() / 2
    };

    assert!(num_elems_per_leaf * F::FIELD_SIZE <= LEAF_BYTES * 8);
    let alphabet_index_in_leaf = *point_to_alphabet % num_elems_per_leaf;

    let left: F = fri_mt_query_alphabet(&query_pair.0, alphabet_index_in_leaf);
    let right: F = fri_mt_query_alphabet(&query_pair.1, alphabet_index_in_leaf);

    if *point_to_alphabet >= codeword_len / 2 {
        *point_to_alphabet -= codeword_len / 2
    }

    (left, right)
}

#[cfg(test)]
mod basefold_utils_test {
    use arith::Field;
    use goldilocks::{Goldilocks, GoldilocksExt2};
    use mersenne31::M31Ext3;

    use crate::fri::utils::copy_elems_to_leaves;

    const BUFFER_LEN: usize = 1 << 10;

    #[test]
    fn test_copy_elems_to_leaves() {
        {
            let m31_ext3_buffer = vec![M31Ext3::ZERO; BUFFER_LEN];
            let leaves = copy_elems_to_leaves(&m31_ext3_buffer);

            const ELEMS_PER_LEAF: usize = 4;

            assert_eq!(m31_ext3_buffer.len() / ELEMS_PER_LEAF, leaves.len());
        }
        {
            let goldilocks_ext2_buffer = vec![GoldilocksExt2::ZERO; BUFFER_LEN];
            let leaves = copy_elems_to_leaves(&goldilocks_ext2_buffer);

            const ELEMS_PER_LEAF: usize = 4;

            assert_eq!(goldilocks_ext2_buffer.len() / ELEMS_PER_LEAF, leaves.len());
        }
        {
            let goldilocks_buffer = vec![Goldilocks::ZERO; BUFFER_LEN];
            let leaves = copy_elems_to_leaves(&goldilocks_buffer);

            const ELEMS_PER_LEAF: usize = 8;

            assert_eq!(goldilocks_buffer.len() / ELEMS_PER_LEAF, leaves.len());
        }
    }
}
