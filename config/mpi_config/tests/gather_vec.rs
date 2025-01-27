use arith::Field;
use mersenne31::M31;
use mpi_config::MPIConfig;

#[test]
fn test_gather_vec() {
    const TEST_SIZE: usize = (1 << 10) + 1;

    let mpi_config = MPIConfig::new();
    let mut local_vec = vec![M31::ZERO; TEST_SIZE];
    for i in 0..TEST_SIZE {
        local_vec[i] = M31::from((mpi_config.world_rank() * TEST_SIZE + i) as u32);
    }

    let mut global_vec = if mpi_config.is_root() {
        vec![M31::ZERO; TEST_SIZE * mpi_config.world_size()]
    } else {
        vec![]
    };

    mpi_config.gather_vec(&local_vec, &mut global_vec);
    if mpi_config.is_root() {
        for (i, v) in global_vec.iter().enumerate() {
            assert_eq!(M31::from(i as u32), *v);
        }
    }

    MPIConfig::finalize();
}
