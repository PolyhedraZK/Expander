use arith::Field;
use mpi::{
    ffi::{MPI_Win, MPI_Win_free},
    topology::Process,
};
use serdes::ExpSerde;

use super::MPISharedMemory;

/// MPI APIs for distributed computing operations
pub trait MPIEngine {
    /// The rank of the root process (always 0)
    const ROOT_RANK: i32 = 0;

    /// Gather vectors from all processes into the root process
    ///
    /// # Arguments
    /// * `local_vec` - The local vector to be gathered from this process
    /// * `global_vec` - Buffer in root process to store all gathered vectors
    ///
    /// # Behavior
    /// - Root process receives all vectors
    /// - Non-root processes send their vectors but don't modify global_vec
    fn gather_vec<F: Sized + Clone>(&self, local_vec: &[F], global_vec: &mut Vec<F>);

    /// Scatter vector from root process into all processes
    ///
    /// # Arguments
    /// * `send_vec` - The global vector to be sent to all processes
    /// * `receive_vec` - Buffer in non-root process to store sent vector segment
    ///
    /// # Behavior
    /// - Root process sends vector segments into all vectors
    /// - Non-root processes receive their segment share but not modifying send_vec
    fn scatter_vec<F: Sized + Clone>(&self, send_vec: &[F], receive_vec: &mut [F]);

    /// Broadcast a field element from root process to all processes
    ///
    /// # Arguments
    /// * `f` - The field element to broadcast (modified in-place)
    ///
    /// # Behavior
    /// - Root process broadcasts its value
    /// - All other processes receive the value
    fn root_broadcast_f<F: Copy>(&self, f: &mut F);

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
    fn sum_vec<F: Field>(&self, local_vec: &[F]) -> Vec<F>;

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
    fn coef_combine_vec<F: Field>(&self, local_vec: &[F], coef: &[F]) -> Vec<F>;

    /// Perform matrix transpose with other MPI processes through MPI all-to-all transpose
    ///
    /// # Arguments
    /// * `row` - The row of elements from the current process among all rows of all processes
    ///
    /// # Behavior
    /// - Each process exchanges chunks of data with every other process
    /// - Resulting data layout on each process swaps one dimension of distribution with another
    ///   (e.g., rows to columns in a distributed matrix)
    fn all_to_all_transpose<F: Sized>(&self, row: &mut [F]);

    /// Gather *variable length* vectors from all processes into the root process
    ///
    /// # Arguments
    /// * `local_vec` - The local variable length vector to be gathered from this process
    /// * `global_vec` - Buffer allocated in the root process to store all gathered vectors
    ///
    /// # Behavior
    /// - Root process receives all *variable length* vectors
    /// - Non-root processes send their vectors but their `global_vec`s are not modified
    ///
    /// # Implementation
    /// Each process serializes their local vector of elements into bytes.
    /// Each process gathers the number of bytes to the root process.
    /// Root process allocates space to collect serialized bytes from each process.
    /// Each process gathers the serialized bytes to the root process.
    /// Root process deserialize all bytes and write into the `global_vec` containing var len elems.
    ///
    /// # NOTE
    /// This method was introduced with a motivation of gathering variable number of Merkle paths
    /// from each processes in Orion PCS.  The `global_vec` is particularly designed to be a vector
    /// of vectors, as we want to keep track of the order of opened Merkle paths on each process,
    /// namely reflecting the order of global Merkle path opening, making the root process easier in
    /// flattening all the Merkle paths into a final flattened vector of opened Merkle paths with
    /// agreeing order from the indices sampled from the Fiat-Shamir RO.
    #[allow(clippy::ptr_arg)]
    fn gather_varlen_vec<F: ExpSerde>(&self, local_vec: &Vec<F>, global_vec: &mut Vec<Vec<F>>);

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

    /// Create a shared memory segment for inter-process communication
    fn create_shared_mem(&self, n_bytes: usize) -> (*mut u8, MPI_Win);

    /// Consume the shared memory segment and create a new shared memory object
    fn consume_obj_and_create_shared<T: MPISharedMemory>(&self, obj: Option<T>) -> (T, MPI_Win) {
        assert!(!self.is_root() || obj.is_some());

        if self.is_root() {
            let obj = obj.unwrap();
            let n_bytes = obj.bytes_size();
            let (mut ptr, window) = self.create_shared_mem(n_bytes);
            let mut ptr_copy = ptr;
            obj.to_memory(&mut ptr_copy);
            self.barrier();
            (T::new_from_memory(&mut ptr), window)
        } else {
            let (mut ptr, window) = self.create_shared_mem(0);
            self.barrier(); // wait for root to write data
            (T::new_from_memory(&mut ptr), window)
        }
    }

    /// Discard the control of shared memory segment
    fn free_shared_mem(&self, window: &mut MPI_Win) {
        unsafe {
            MPI_Win_free(window as *mut MPI_Win);
        }
    }
}
