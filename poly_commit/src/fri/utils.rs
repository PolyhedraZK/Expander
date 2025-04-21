use arith::{FFTField, Field};
use tree::{Leaf, LEAF_BYTES};

pub type FRICommitment = tree::Node;

#[derive(Clone, Debug, Default)]
pub struct FRIScratchPad<F: FFTField> {
    pub reed_solomon_commitment: tree::Tree,
    pub codeword: Vec<F>,
}

unsafe impl<F: FFTField> Send for FRIScratchPad<F> {}

#[allow(unused)]
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
        .map(|elems_chunk| unsafe {
            let mut leaf = Leaf::default();

            let u8_cast_slice = std::slice::from_raw_parts(
                elems_chunk.as_ptr() as *const u8,
                num_elems_per_leaf * field_bytes,
            );
            leaf.data[..u8_cast_slice.len()].copy_from_slice(u8_cast_slice);

            leaf
        })
        .collect()
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
