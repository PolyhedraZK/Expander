use std::sync::Arc;

use crate::AtomicVec;

/// Configuration for MPI
/// Assumptions
/// 1. Each thread writes to its own local memory
/// 2. Each thread reads from all other threads' local memory
/// 3. All threads have the same global memory
/// 4. IMPORTANT!!! The threads are synchronized by the caller; within each period of time, all
///    threads write a same amount of data
#[derive(Debug, Clone)]
pub struct ThreadConfig {
    pub world_rank: i32,                  // indexer for the thread
    pub local_memory: Arc<AtomicVec<u8>>, // local memory for the thread
}

impl Default for ThreadConfig {
    #[inline]
    fn default() -> Self {
        Self {
            world_rank: 0,
            local_memory: Arc::new(AtomicVec::new(0)),
        }
    }
}

impl PartialEq for ThreadConfig {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // equality is based on rank and size
        // it doesn't check the memory are consistent
        self.world_rank == other.world_rank
    }
}

impl ThreadConfig {
    #[inline]
    pub fn new(world_rank: i32, buffer_size: usize) -> Self {
        Self {
            world_rank,
            local_memory: Arc::new(AtomicVec::new(buffer_size)),
        }
    }

    #[inline]
    pub fn is_root(&self) -> bool {
        self.world_rank == 0
    }

    #[inline]
    pub fn append(&self, data: &[u8]) -> Result<usize, &'static str> {
        self.local_memory
            .append(data)
            .ok_or("Failed to append: insufficient capacity")
    }

    #[inline]
    pub fn read(&self, start: usize, end: usize) -> &[u8] {
        self.local_memory
            .get_slice(start, end)
            .ok_or(format!(
                "failed to read between {start} and {end} for slice of length {}",
                self.local_memory.len()
            ))
            .unwrap()
    }

    #[inline]
    /// Get the length of local memory
    pub fn size(&self) -> usize {
        self.local_memory.len()
    }
}
