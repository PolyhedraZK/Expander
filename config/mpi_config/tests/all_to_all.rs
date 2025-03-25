use arith::Field;
use ark_std::test_rng;
use gf2::{GF2x128, GF2x64, GF2x8};
use itertools::izip;
use mersenne31::{M31Ext3, M31x16, M31};
use mpi_config::MPIConfig;

fn test_all_to_all_transpose_helper<F: Field>(mpi_config: &MPIConfig) {
    const TEST_MATRIX_LEN: usize = 1 << 22;

    dbg!(F::NAME);
    dbg!(F::SIZE);

    let mut rng = test_rng();
    let global_matrix: Vec<_> = (0..TEST_MATRIX_LEN)
        .map(|_| F::random_unsafe(&mut rng))
        .collect();

    let local_length = TEST_MATRIX_LEN / mpi_config.world_size();

    let local_share_starts = local_length * mpi_config.world_rank();
    let mut local_shares =
        global_matrix[local_share_starts..local_length + local_share_starts].to_vec();

    dbg!(local_share_starts, local_length);

    mpi_config.all_to_all_transpose(&mut local_shares);

    let transpose_slice_len = local_length / mpi_config.world_size();
    izip!(
        global_matrix.chunks(local_length),
        local_shares.chunks(transpose_slice_len)
    )
    .enumerate()
    .for_each(|(i, (c, ls))| {
        let row_starts = transpose_slice_len * mpi_config.world_rank();

        dbg!(i, row_starts);

        izip!(&c[row_starts..transpose_slice_len + row_starts], ls)
            .for_each(|(left, right)| assert_eq!(*left, *right));
    });
}

#[test]
fn test_all_to_all_transpose() {
    let mpi_config = MPIConfig::new();

    test_all_to_all_transpose_helper::<GF2x128>(&mpi_config);
    test_all_to_all_transpose_helper::<GF2x64>(&mpi_config);
    test_all_to_all_transpose_helper::<GF2x8>(&mpi_config);

    test_all_to_all_transpose_helper::<M31x16>(&mpi_config);
    test_all_to_all_transpose_helper::<M31>(&mpi_config);
    test_all_to_all_transpose_helper::<M31Ext3>(&mpi_config);

    MPIConfig::finalize();
}
