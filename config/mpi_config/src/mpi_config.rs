use std::sync::Arc;

use crate::{ThreadConfig, MAX_WAIT_CYCLES};

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
    pub global_memory: Arc<[u8]>,
    pub threads: Vec<ThreadConfig>,
}

impl Default for MPIConfig {
    #[inline]
    fn default() -> Self {
        Self {
            world_size: 1,
            global_memory: Arc::from(vec![]),
            threads: vec![],
        }
    }
}

impl PartialEq for MPIConfig {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // equality is based on size
        // it doesn't check the memory are consistent
        self.world_size == other.world_size
    }
}

impl MPIConfig {
    #[inline]
    pub fn new(world_size: i32, global_data: Arc<[u8]>, buffer_size: usize) -> Self {
        Self {
            world_size,
            global_memory: global_data,
            threads: (0..world_size)
                .map(|rank| ThreadConfig::new(rank, buffer_size))
                .collect(),
        }
    }

    #[inline]
    pub fn world_size(&self) -> i32 {
        self.world_size
    }

    /// Sync with all threads' local memory by waiting until there is new data to read from all
    /// threads.
    /// Returns a vector of slices, one for each thread's new data
    pub fn sync(&self, start: usize, end: usize) -> Vec<&[u8]> {
        let total = self.threads.len();
        let mut pending = (0..total).collect::<Vec<_>>();
        let mut results: Vec<&[u8]> = vec![&[]; total];
        let mut wait_cycles = 0;

        // Keep going until we've read from all threads
        while !pending.is_empty() {
            // Use retain to avoid re-checking already synced threads
            pending.retain(|&i| {
                let len = self.threads[i].size();
                if len >= end {
                    results[i] = self.threads[i].read(start, end);
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
