use std::sync::Arc;

use arith::Field;

use crate::{ThreadConfig, MAX_WAIT_CYCLES};

/// Configuration for MPI
/// Assumptions
/// 1. Each thread writes to its own local memory
/// 2. Each thread reads from all other threads' local memory
/// 3. All threads have the same global memory
/// 4. IMPORTANT!!! The threads are synchronized by the caller; within each period of time, all
///    threads write a same amount of data
///
/// The config struct only uses pointers so we avoid cloning of all data
#[derive(Debug, Clone)]
pub struct MPIConfig {
    pub world_size: i32,            // Number of threads
    pub global_memory: Arc<[u8]>,   // Global memory shared by all threads
    pub threads: Vec<ThreadConfig>, // Local memory for each thread
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

    #[inline]
    /// check the caller's thread is the root thread
    pub fn is_root(&self) -> bool {
        rayon::current_thread_index().unwrap() == 0
    }

    #[inline]
    /// Get the current thread
    pub fn current_thread(&self) -> &ThreadConfig {
        let index = rayon::current_thread_index().unwrap();
        &self.threads[index]
    }

    #[inline]
    /// Get the current thread
    pub fn current_thread_mut(&mut self) -> &mut ThreadConfig {
        let index = rayon::current_thread_index().unwrap();
        &mut self.threads[index]
    }

    #[inline]
    /// Get the size of the current local memory
    pub fn current_size(&self) -> usize {
        self.current_thread().size()
    }

    #[inline]
    /// Check if the current thread is synced
    pub fn is_current_thread_synced(&self) -> bool {
        self.current_thread().is_synced()
    }

    #[inline]
    /// Check if all threads are synced
    pub fn are_all_threads_synced(&self) -> bool {
        self.threads.iter().all(|t| t.is_synced())
    }

    #[inline]
    /// Sync up the current thread
    /// Returns a vector of slices, one for each thread's new data
    pub fn sync_up(&mut self) -> Vec<&[u8]> {
        if self.is_current_thread_synced() {
            return vec![];
        }

        let start = self.current_thread().last_synced;
        let end = self.current_thread().size();
        // update the pointer to the latest index
        self.current_thread_mut().last_synced = end;
        let result = self.read_all(start, end);

        result
    }

    /// Read all threads' local memory by waiting until there is new data to read from all
    /// threads.
    /// Returns a vector of slices, one for each thread's new data
    /// Update the sync pointer of the current thread
    ///
    /// The threads are synchronized by the caller; within each period of time, all
    /// threads write a same amount of data
    pub fn read_all(&self, start: usize, end: usize) -> Vec<&[u8]> {
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

    #[inline]
    // todo: add a field buffer to the thread config so we can avoid field (de)serialization
    pub fn read_all_field<F: Field>(&self, start: usize, end: usize) -> Vec<Vec<F>> {
        let data = self.read_all(start, end);
        data.iter()
            .map(|x| {
                x.chunks(F::SIZE)
                    .map(|y| F::deserialize_from(y).unwrap())
                    .collect()
            })
            .collect()
    }

    #[inline]
    pub fn read_all_field_flat<F: Field>(&self, start: usize, end: usize) -> Vec<F> {
        let data = self.read_all(start, end);
        data.iter()
            .flat_map(|x| x.chunks(F::SIZE).map(|y| F::deserialize_from(y).unwrap()))
            .collect()
    }

    #[inline]
    /// Append data to the current thread's local memory
    pub fn append_local(&self, data: &[u8]) {
        let thread = self.current_thread();
        thread.append(data).expect("Failed to append");
    }

    #[inline]
    /// Append data to the current thread's local memory
    pub fn append_local_field<F: Field>(&self, f: &F) {
        let mut data = vec![];
        f.serialize_into(&mut data).unwrap();
        self.append_local(&data);
    }

    /// coefficient has a length of mpi_world_size
    #[inline]
    pub fn coef_combine_vec<F: Field>(&self, local_vec: &Vec<F>, coefficient: &[F]) -> Vec<F> {
        if self.world_size == 1 {
            // Warning: literally, it should be coefficient[0] * local_vec
            // but coefficient[0] is always one in our use case of self.world_size = 1
            local_vec.clone()
        } else {
            // write local vector to the buffer, then sync up all threads
            let start = self.current_thread().size();
            let data = local_vec
                .iter()
                .flat_map(|&x| {
                    let mut buf = vec![];
                    x.serialize_into(&mut buf).unwrap();
                    buf
                })
                .collect::<Vec<u8>>();
            self.append_local(&data);
            let end = self.current_thread().size();
            let all_fields = self.read_all_field::<F>(start, end);

            // build the result via linear combination
            let mut result = vec![F::zero(); local_vec.len()];
            for i in 0..local_vec.len() {
                for j in 0..(self.world_size as usize) {
                    result[i] += all_fields[j][i] * coefficient[j];
                }
            }

            result
        }
    }

    #[inline]
    /// Finalize function does nothing except for a minimal sanity check
    /// that all threads have the same amount of data
    pub fn finalize(&self) {
        let len = self.threads[0].size();
        self.threads.iter().skip(1).for_each(|t| {
            assert_eq!(t.size(), len);
        });

        // do nothing
    }
}
