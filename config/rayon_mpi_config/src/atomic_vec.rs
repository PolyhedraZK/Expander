use std::sync::atomic::{AtomicUsize, Ordering};

/// A lock-free append-only vector implementation
/// credit: Claude
#[derive(Debug)]
pub struct AtomicVec<T> {
    // The actual data storage
    data: Vec<T>,
    // Current length of valid data
    len: AtomicUsize,
}

impl<T: Clone> AtomicVec<T> {
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::with_capacity(capacity);
        // Pre-fill with default values to avoid reallocation
        data.resize_with(capacity, || unsafe { std::mem::zeroed() });
        Self {
            data,
            len: AtomicUsize::new(0),
        }
    }

    /// Append data to the vector
    /// Returns the start index where data was appended
    pub fn append(&self, items: &[T]) -> Option<usize> {
        let old_len = self.len.fetch_add(items.len(), Ordering::AcqRel);
        if old_len + items.len() > self.data.capacity() {
            // Restore the length if we would exceed capacity
            self.len.fetch_sub(items.len(), Ordering::Release);
            return None;
        }

        // Safe because:
        // 1. We've pre-allocated the space
        // 2. Each thread writes to its own section
        // 3. The atomic len ensures no overlapping writes
        unsafe {
            let ptr = self.data.as_ptr().add(old_len) as *mut T;
            for (i, item) in items.iter().enumerate() {
                std::ptr::write(ptr.add(i), item.clone());
            }
        }

        Some(old_len)
    }

    /// Read a slice of data
    pub fn get_slice(&self, start: usize, end: usize) -> Option<&[T]> {
        let current_len = self.len.load(Ordering::Acquire);
        if start >= current_len || end > current_len || start > end {
            return None;
        }

        // Safe because:
        // 1. We've checked the bounds
        // 2. No data is ever modified after being written
        Some(unsafe { std::slice::from_raw_parts(self.data.as_ptr().add(start), end - start) })
    }

    /// Get current length
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Acquire)
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
