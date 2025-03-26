use arith::Field;
use mpi::topology::Process;

/// MPI APIs
pub trait MPIEngine {
    const ROOT_RANK: i32;

    /// The communication limit for MPI is 2^30. Save 10 bits for #parties here.
    const CHUNK_SIZE: usize;

    /// Initialize the MPI environment
    // OK if already initialized, mpi::initialize() will return None
    fn init();

    /// Finalize the MPI environment
    fn finalize();

    /// Create a new MPI engine for the prover
    fn prover_new() -> Self;

    /// Create a new MPI engine for the verifier
    fn verifier_new(world_size: i32) -> Self;

    /// Gather a vector from all the processes into the root process
    fn gather_vec<F: Sized + Clone>(&self, local_vec: &Vec<F>, global_vec: &mut Vec<F>);

    /// Root process broadcast a value f into all the processes
    fn root_broadcast_f<F: Field>(&self, f: &mut F);

    /// Root process broadcast a vector of bytes into all the processes
    fn root_broadcast_bytes(&self, bytes: &mut Vec<u8>);

    /// sum up all local values
    fn sum_vec<F: Field>(&self, local_vec: &Vec<F>) -> Vec<F>;

    /// coef has a length of mpi_world_size
    fn coef_combine_vec<F: Field>(&self, local_vec: &Vec<F>, coef: &[F]) -> Vec<F>;

    /// Check if there is only one process in the world
    fn is_single_process(&self) -> bool;

    /// Get the number of processes in the world
    fn world_size(&self) -> usize;

    /// Get the rank of the current process
    fn world_rank(&self) -> usize;

    #[inline(always)]
    /// Check if the current process is the root process
    fn is_root(&self) -> bool {
        self.world_rank() == Self::ROOT_RANK as usize
    }

    /// Get the root process
    fn root_process(&self) -> Process;

    /// Barrier for all the processes
    fn barrier(&self);
}
