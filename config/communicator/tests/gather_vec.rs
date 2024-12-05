use arith::Field;
use communicator::{ExpanderComm, MPICommunicator};
use mersenne31::M31;

#[test]
fn test_gather_vec() {
    const TEST_SIZE: usize = (1 << 10) + 1;

    let mpi_communicator = MPICommunicator::new(2);
    let mut local_vec = vec![M31::ZERO; TEST_SIZE];
    for i in 0..TEST_SIZE {
        local_vec[i] = M31::from((mpi_communicator.world_rank() * TEST_SIZE + i) as u32);
    }

    let mut global_vec = if mpi_communicator.is_root() {
        vec![M31::ZERO; TEST_SIZE * mpi_communicator.world_size()]
    } else {
        vec![]
    };

    mpi_communicator.gather_vec(&local_vec, &mut global_vec);
    if mpi_communicator.is_root() {
        for (i, v) in global_vec.iter().enumerate() {
            assert_eq!(M31::from(i as u32), *v);
        }
    }

    MPICommunicator::finalize();
}
