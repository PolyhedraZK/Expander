use std::sync::Arc;

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serial_test::serial;

use crate::{MPIConfig, ThreadConfig};

// Example usage
#[test]
#[serial]
fn test_single_thread() {
    let config = ThreadConfig::new(0, 1024 * 1024);

    // Append some data
    let pos = config.append(&[1, 2, 3, 4]).unwrap();
    println!("Appended at position: {}", pos);

    // Read it back
    let data = config.read(pos, pos + 4);
    println!("Read back: {:?}", data);
}

#[test]
#[serial]
// Assuming we have the MPIConfig and AtomicVec from previous example
fn test_parallel_processing() {
    // Create some test data for global memory
    let global_data: Arc<[u8]> = Arc::from((0..1024).map(|i| i as u8).collect::<Vec<u8>>());
    let num_threads = rayon::current_num_threads();

    // Create configs for all threads
    let mpi_config = MPIConfig::new(num_threads as i32, global_data, 1024 * 1024);

    // Process in parallel using rayon
    (0..num_threads).into_par_iter().for_each(|rank| {
        let thread = &mpi_config.threads[rank];

        // Simulate some work: read from global memory and write to local
        // Each thread reads a different section of global memory
        let chunk_size = mpi_config.global_memory.len() / num_threads;
        let start = rank * chunk_size;
        let end = if rank == num_threads - 1 {
            mpi_config.global_memory.len()
        } else {
            start + chunk_size
        };

        // Read from global memory
        if let Some(global_chunk) = mpi_config.global_memory.get(start..end) {
            // Process the data (example: multiply each byte by rank + 1)
            let processed: Vec<u8> = global_chunk
                .iter()
                .map(|&x| x.wrapping_mul((rank + 1) as u8))
                .collect();

            // Write to local memory
            match thread.append(&processed) {
                Ok(pos) => println!(
                    "Thread {} wrote {} bytes at position {}",
                    rank,
                    processed.len(),
                    pos
                ),
                Err(e) => eprintln!("Thread {} failed to write: {}", rank, e),
            }
        }
    });

    // Verify results
    for rank in 0..num_threads {
        let thread = &mpi_config.threads[rank];
        let data = thread.local_memory.get_slice(0, thread.local_memory.len());

        if let Some(local_data) = data {
            println!(
                "Thread {} final local memory size: {}",
                rank,
                local_data.len()
            );
            // Print first few bytes for verification
            if !local_data.is_empty() {
                println!(
                    "Thread {} first few bytes: {:?}",
                    rank,
                    &local_data[..local_data.len().min(4)]
                );
            }
        }
    }
}

#[test]
#[serial]
fn test_cross_thread_communication() {
    // Create global data
    let global_data: Arc<[u8]> = Arc::from((0..16).map(|i| i as u8).collect::<Vec<u8>>());
    let num_threads = rayon::current_num_threads();
    let data_len = 4;

    // Create configs for all threads
    let mpi_config = MPIConfig::new(num_threads as i32, global_data, 1024 * 1024);

    let expected_result = (0..num_threads)
        .map(|i| vec![i as u8 + 1; data_len])
        .collect::<Vec<_>>();

    // write to its own memory, and read from all others
    (0..num_threads).into_par_iter().for_each(|rank| {
        let thread = &mpi_config.threads[rank];

        let data = vec![rank as u8 + 1; data_len];
        let start = thread.size();
        let end = start + data_len;

        thread.append(&data).expect("Failed to append");

        let results = mpi_config.read_all(start, end);
        assert_eq!(results.len(), num_threads as usize);

        for (i, result) in results.iter().enumerate() {
            assert_eq!(result.len(), data_len);
            assert_eq!(result, &expected_result[i]);
        }
    });
}

#[test]
#[serial]
fn test_incremental_updates() {
    let global_data = Arc::<[u8]>::from(vec![0u8; 64]);
    let num_threads = rayon::current_num_threads();
    let data_len = 4;

    let expected_result = (0..num_threads)
        .map(|i| vec![i as u8 + 1; data_len])
        .collect::<Vec<_>>();

    // Create configs for all threads
    let mpi_config = MPIConfig::new(num_threads as i32, global_data, 1024 * 1024);

    // write to its own memory, and read from all others
    (0..num_threads).into_par_iter().for_each(|rank| {
        // 10 interactions among the threads; without spawning and killing new threads
        // during each interaction, a fixed amount of data will be written to each thead's local
        // memory
        for i in 0..10 {
            let thread = &mpi_config.threads[rank];
            let data = vec![((rank + 1) * (i + 1)) as u8; data_len];
            let start = thread.size();
            let end = start + data_len;

            thread.append(&data).expect("Failed to append");

            let results = mpi_config.read_all(start, end);
            assert_eq!(results.len(), num_threads as usize);

            println!("Thread {} iteration {}: {:?}", rank, i, results);

            for (j, result) in results.iter().enumerate() {
                assert_eq!(result.len(), data_len as usize);
                result.iter().zip(&expected_result[j]).for_each(|(&a, &b)| {
                    assert_eq!(a, b * (i + 1) as u8);
                });
            }
        }
    });
}
