use arith::Field;
use mpi::topology::Process;

/// MPI APIs for distributed computing operations
pub trait MPIEngine {
    /// The rank of the root process (always 0)
    const ROOT_RANK: i32;

    /// The maximum chunk size for MPI communications 
    const CHUNK_SIZE: usize;

    /// Initialize the MPI environment.
    /// Safe to call multiple times as `mpi::initialize()` will return None if already initialized.
    fn init();

    /// Finalize the MPI environment
    fn finalize();

    /// Create a new MPI engine for the prover
    fn prover_new() -> Self;

    /// Create a new MPI engine for the verifier with specified world size
    /// 
    /// # Arguments
    /// * `world_size` - The total number of processes in the MPI world
    fn verifier_new(world_size: i32) -> Self;

    /// Gather vectors from all processes into the root process
    /// 
    /// # Arguments
    /// * `local_vec` - The local vector to be gathered from this process
    /// * `global_vec` - Buffer in root process to store all gathered vectors
    /// 
    /// # Behavior
    /// - Root process receives all vectors
    /// - Non-root processes send their vectors but don't modify global_vec
    fn gather_vec<F: Sized + Clone>(&self, local_vec: &Vec<F>, global_vec: &mut Vec<F>);

    /// Broadcast a field element from root process to all processes
    /// 
    /// # Arguments
    /// * `f` - The field element to broadcast (modified in-place)
    /// 
    /// # Behavior
    /// - Root process broadcasts its value
    /// - All other processes receive the value
    fn root_broadcast_f<F: Field>(&self, f: &mut F);

    /// Broadcast a vector of bytes from root process to all processes
    /// 
    /// # Arguments
    /// * `bytes` - The byte vector to broadcast (modified in-place)
    /// 
    /// # Behavior
    /// - Root process broadcasts its bytes
    /// - All other processes receive the bytes
    fn root_broadcast_bytes(&self, bytes: &mut Vec<u8>);

    /// Sum up field elements across all processes
    /// 
    /// # Arguments
    /// * `local_vec` - The local vector of field elements to sum
    /// 
    /// # Returns
    /// A vector containing the sum of corresponding elements from all processes
    fn sum_vec<F: Field>(&self, local_vec: &Vec<F>) -> Vec<F>;

    /// Combines vectors from all MPI processes using weighted coefficients
    /// 
    /// # Arguments
    /// * `local_vec` - The local vector from the current process
    /// * `coef` - Array of coefficients, with length equal to world_size
    /// 
    /// # Returns
    /// * For single process: Returns local_vec.clone()
    /// * For root process: Returns weighted combination Σ(coef[j] * process_j_vector[i])
    /// * For other processes: Returns zero vector of same length
    /// 
    /// # Implementation
    /// Root process gathers all vectors and computes the weighted sum.
    /// Non-root processes participate in gathering but return zero vectors.
    fn coef_combine_vec<F: Field>(&self, local_vec: &Vec<F>, coef: &[F]) -> Vec<F>;

    /// Check if there is only one process in the MPI world
    fn is_single_process(&self) -> bool;

    /// Get the total number of processes in the MPI world
    fn world_size(&self) -> usize;

    /// Get the rank of the current process
    fn world_rank(&self) -> usize;

    /// Check if the current process is the root process
    #[inline(always)]
    fn is_root(&self) -> bool {
        self.world_rank() == Self::ROOT_RANK as usize
    }

    /// Get the root process handle
    fn root_process(&self) -> Process;

    /// Synchronize all processes at this point
    fn barrier(&self);
}
