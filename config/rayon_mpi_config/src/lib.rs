//! This module implements a synchronized MPI configuration for Rayon.
//!
//! Assumptions
//! 1. There will NOT be a root process that collects data from all other processes and broadcast
//!    it.
//! 2. Each thread writes to its own local memory.
//! 3. Each thread reads from all other threads' local memory.
//! 4. All threads have access to a same global, read-only memory. This global memory is initialized
//!    before the threads start and will remain invariant during the threads' execution.
//! 5. IMPORTANT!!! The threads are synchronized by the caller; within each period of time, all
//!    threads write a same amount of data

mod atomic_vec;
pub use atomic_vec::AtomicVec;

mod mpi_config;
pub use mpi_config::MPIConfig;

/// Max number of std::hint::spin_loop() we will do before panicking
// TODO: for ARMs this number may need to be adjusted
const MAX_WAIT_CYCLES: usize = 1000000;

#[cfg(test)]
mod tests;
