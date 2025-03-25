use mpi_config::MPIConfig;

#[test]
fn test_gather_varlen_vec() {
    let mpi_config = MPIConfig::new();

    let msg: Vec<_> = (0..=mpi_config.world_rank()).collect();
    let mut global_elems: Vec<Vec<usize>> = Vec::new();

    mpi_config.gather_varlen_vec(&msg, &mut global_elems);

    dbg!(&global_elems);

    MPIConfig::finalize();

    global_elems.iter().enumerate().for_each(|(i, elems)| {
        (0..=i).for_each(|j| {
            assert_eq!(j, elems[j]);
        })
    });
}
