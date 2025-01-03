use std::sync::Arc;

use crate::{AtomicVec, MAX_WAIT_CYCLES};

/// Configuration for MPI
/// Assumptions
/// 1. Each thread writes to its own local memory
/// 2. Each thread reads from all other threads' local memory
/// 3. All threads have the same global memory
/// 4. IMPORTANT!!! The threads are synchronized by the caller; within each period of time, all
///    threads write a same amount of data
#[derive(Debug)]
pub struct MPIConfig {
    pub world_size: i32,
    pub world_rank: i32,
    pub global_memory: Arc<[u8]>,
    pub local_memory: Arc<AtomicVec<u8>>,
}

impl Default for MPIConfig {
    fn default() -> Self {
        Self {
            world_size: 1,
            world_rank: 0,
            global_memory: Arc::from(vec![]),
            local_memory: Arc::new(AtomicVec::new(0)),
        }
    }
}

impl PartialEq for MPIConfig {
    fn eq(&self, other: &Self) -> bool {
        // equality is based on rank and size
        // it doesn't check the memory are consistent
        self.world_rank == other.world_rank && self.world_size == other.world_size
    }
}

impl MPIConfig {
    pub fn new(
        world_size: i32,
        world_rank: i32,
        global_data: Arc<[u8]>,
        buffer_size: usize,
    ) -> Self {
        Self {
            world_size,
            world_rank,
            global_memory: global_data,
            local_memory: Arc::new(AtomicVec::new(buffer_size)),
        }
    }

    pub fn append_local(&self, data: &[u8]) -> Result<usize, &'static str> {
        self.local_memory
            .append(data)
            .ok_or("Failed to append: insufficient capacity")
    }

    pub fn read_local(&self, start: usize, end: usize) -> &[u8] {
        self.local_memory
            .get_slice(start, end)
            .ok_or(format!(
                "failed to read between {start} and {end} for slice of length {}",
                self.local_memory.len()
            ))
            .unwrap()
    }

    /// Get the length of local memory
    pub fn local_len(&self) -> usize {
        self.local_memory.len()
    }

    /// Sync with all threads' local memory by waiting until there is new data to read from all
    /// threads.
    /// Returns a vector of slices, one for each thread's new data
    pub fn sync_all<'a>(threads: &'a [MPIConfig], start: usize, end: usize) -> Vec<&'a [u8]> {
        let total = threads.len();
        let mut pending = (0..total).collect::<Vec<_>>();
        let mut results: Vec<&'a [u8]> = vec![&[]; total];
        let mut wait_cycles = 0;

        // Keep going until we've read from all threads
        while !pending.is_empty() {
            // Use retain to avoid re-checking already synced threads
            pending.retain(|&i| {
                let len = threads[i].local_len();
                if len >= end {
                    results[i] = threads[i].read_local(start, end);
                    false // Remove from pending
                } else {
                    true // Keep in pending
                }
            });

            if !pending.is_empty() {
                // Claude suggest to use the following approach for waiting
                //
                // Simple spin - Rayon manages the thread pool efficiently
                // Hint to the CPU that we're spinning (reduces power consumption)
                // - For AMD/Intel it delays for 140 cycles
                // - For ARM it is 1~2 cycles (We may need to manually adjust MAX_WAIT_CYCLES)
                std::hint::spin_loop();
                wait_cycles += 1;
                if wait_cycles > MAX_WAIT_CYCLES {
                    panic!("Exceeded max wait cycles");
                }
            }
        }
        results
    }
}
