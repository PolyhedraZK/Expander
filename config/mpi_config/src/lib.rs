use std::{cmp, fmt::Debug};

use arith::Field;
// use mpi::{
//     environment::Universe,
//     ffi,
//     topology::{Process, SimpleCommunicator},
//     traits::*,
// };

#[macro_export]
macro_rules! root_println {
    ($config: expr, $($arg:tt)*) => {
        if $config.is_root() {
            println!($($arg)*);
        }
    };
}

// static mut UNIVERSE: Option<Universe> = None;
// static mut WORLD: Option<SimpleCommunicator> = None;

#[derive(Clone, Debug, PartialEq)]
pub struct MPIConfig {
    /// The shared memory between all the processes
    pub universe: Vec<u8>,
    /// The local memory of the current process
    pub worlds: Vec<Vec<u8>>,
    // pub universe: Option<&'static mpi::environment::Universe>,
    // pub world: Option<&'static SimpleCommunicator>,
    /// The number of worlds
    pub world_size: i32,
    /// The current world rank
    pub world_rank: i32,
}

impl Default for MPIConfig {
    fn default() -> Self {
        Self {
            universe: Vec::new(),
            worlds: Vec::new(),
            world_size: 1,
            world_rank: 0,
        }
    }
}

// impl Debug for MPIConfig {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let universe_fmt = if self.universe.is_none() {
//             Option::<usize>::None
//         } else {
//             Some(self.universe.unwrap().buffer_size())
//         };

//         let world_fmt = if self.world.is_none() {
//             Option::<usize>::None
//         } else {
//             Some(0usize)
//         };

//         f.debug_struct("MPIConfig")
//             .field("universe", &universe_fmt)
//             .field("world", &world_fmt)
//             .field("world_size", &self.world_size)
//             .field("world_rank", &self.world_rank)
//             .finish()
//     }
// }

// // Note: may not be correct
// impl PartialEq for MPIConfig {
//     fn eq(&self, other: &Self) -> bool {
//         self.world_rank == other.world_rank && self.world_size == other.world_size
//     }
// }

/// MPI toolkit:
impl MPIConfig {
    const ROOT_RANK: i32 = 0;

    // /// The communication limit for MPI is 2^30. Save 10 bits for #parties here.
    // const CHUNK_SIZE: usize = 1usize << 20;

    // OK if already initialized, mpi::initialize() will return None
    #[allow(static_mut_refs)]
    pub fn init() {
        // do nothing


       

        // unsafe {
        //     let universe = mpi::initialize();
        //     if universe.is_some() {
        //         UNIVERSE = universe;
        //         WORLD = Some(UNIVERSE.as_ref().unwrap().world());
        //     }
        // }
    }

    #[inline]
    pub fn finalize() {
        // do nothing
        // unsafe { ffi::MPI_Finalize() };
    }

    #[allow(static_mut_refs)]
    pub fn new() -> Self {
        Self::init();
        // let universe = unsafe { UNIVERSE.as_ref() };
        // let world = unsafe { WORLD.as_ref() };
        // let world_size = if let Some(world) = world {
        //     world.size()
        // } else {
        //     1
        // };
        // let world_rank = if let Some(world) = world {
        //     world.rank()
        // } else {
        //     0
        // };
        // Self {
        //     universe,
        //     world,
        //     world_size,
        //     world_rank,
        // }

         let num_worlds = rayon::current_num_threads() as i32;
        let world_rank = match rayon::current_thread_index() {
            Some(rank) => rank as i32,
            None => 0,
        };

        let universe = vec![];
        let worlds = vec![vec![]; num_worlds as usize];

        Self {
            universe,
            worlds,
            world_size: num_worlds,
            world_rank,
        }
    }

    #[inline]
    pub fn new_for_verifier(world_size: i32) -> Self {

        let universe = vec![];
        let worlds = vec![vec![]; world_size as usize];

        Self {
            universe,
            worlds,
            world_size: world_size,
            world_rank: 0,
        }

        // Self {
        //     universe: None,
        //     world: None,
        //     world_size,
        //     world_rank: 0,
        // }
    }

    /// Return an u8 vector sharing THE SAME MEMORY SLOT with the input.
    #[inline]
    unsafe fn elem_to_u8_bytes<V: Sized>(elem: &V, byte_size: usize) -> Vec<u8> {
        Vec::<u8>::from_raw_parts((elem as *const V) as *mut u8, byte_size, byte_size)
    }

    /// Return an u8 vector sharing THE SAME MEMORY SLOT with the input.
    #[inline]
    unsafe fn vec_to_u8_bytes<F: Field>(vec: &Vec<F>) -> Vec<u8> {
        Vec::<u8>::from_raw_parts(
            vec.as_ptr() as *mut u8,
            vec.len() * F::SIZE,
            vec.capacity() * F::SIZE,
        )
    }

    #[allow(clippy::collapsible_else_if)]
    pub fn gather_vec<F: Field>(&mut self, local_vec: &Vec<F>, global_vec: &mut Vec<F>) {
        if self.world_size == 1 {
            *global_vec = local_vec.clone();
            return;
        }
    
        // For non-root processes, we just need to store our local vector in our world's memory
        if !self.is_root() {
            assert!(global_vec.is_empty(), "Non-root processes should have empty global vectors");
            
            // Convert local vector to bytes and store in our world's slot
            unsafe {
                let local_vec_u8 = Self::vec_to_u8_bytes(local_vec);
                self.worlds[self.world_rank as usize] = local_vec_u8;
            }
            return;
        }
    
        // For root process, we need to gather all vectors
        assert!(
            global_vec.len() == local_vec.len() * self.world_size(),
            "Root's global_vec size must match total data size"
        );
    
        // First, store root's local vector in the beginning of global_vec
        let root_data_size = local_vec.len();
        global_vec[..root_data_size].copy_from_slice(local_vec);
    
        // Then gather data from other processes
        for rank in 1..self.world_size {
            let rank = rank as usize;
            let start_idx = rank * root_data_size;
            let end_idx = start_idx + root_data_size;
    
            // Get the bytes from the corresponding world's memory
            let world_bytes = &self.worlds[rank];
            
            // Safety: We're reconstructing the vector with the same layout
            unsafe {
                let other_vec = Vec::<F>::from_raw_parts(
                    world_bytes.as_ptr() as *mut F,
                    root_data_size,
                    root_data_size,
                );
                
                // Copy the data to the appropriate position in global_vec
                global_vec[start_idx..end_idx].copy_from_slice(&other_vec);
                
                // Don't drop the vector since we don't own the memory
                std::mem::forget(other_vec);
            }
        }

        // unsafe {
        //     if self.world_size == 1 {
        //         *global_vec = local_vec.clone()
        //     } else {
        //         assert!(!self.is_root() || global_vec.len() == local_vec.len() * self.world_size());

        //         let local_vec_u8 = Self::vec_to_u8_bytes(local_vec);
        //         let local_n_bytes = local_vec_u8.len();
        //         let n_chunks = (local_n_bytes + Self::CHUNK_SIZE - 1) / Self::CHUNK_SIZE;
        //         if n_chunks == 1 {
        //             if self.world_rank == Self::ROOT_RANK {
        //                 let mut global_vec_u8 = Self::vec_to_u8_bytes(global_vec);
        //                 self.root_process()
        //                     .gather_into_root(&local_vec_u8, &mut global_vec_u8);
        //                 global_vec_u8.leak(); // discard control of the memory
        //             } else {
        //                 self.root_process().gather_into(&local_vec_u8);
        //             }
        //         } else {
        //             if self.world_rank == Self::ROOT_RANK {
        //                 let mut chunk_buffer_u8 = vec![0u8; Self::CHUNK_SIZE * self.world_size()];
        //                 let mut global_vec_u8 = Self::vec_to_u8_bytes(global_vec);
        //                 for i in 0..n_chunks {
        //                     let local_start = i * Self::CHUNK_SIZE;
        //                     let local_end = cmp::min(local_start + Self::CHUNK_SIZE, local_n_bytes);
        //                     self.root_process().gather_into_root(
        //                         &local_vec_u8[local_start..local_end],
        //                         &mut chunk_buffer_u8,
        //                     );

        //                     // distribute the data to where they belong to in global vec
        //                     let actual_chunk_size = local_end - local_start;
        //                     for j in 0..self.world_size() {
        //                         let global_start = j * local_n_bytes + local_start;
        //                         let global_end = global_start + actual_chunk_size;
        //                         global_vec_u8[global_start..global_end].copy_from_slice(
        //                             &chunk_buffer_u8[j * Self::CHUNK_SIZE
        //                                 ..j * Self::CHUNK_SIZE + actual_chunk_size],
        //                         );
        //                     }
        //                 }
        //                 global_vec_u8.leak(); // discard control of the memory
        //             } else {
        //                 for i in 0..n_chunks {
        //                     let local_start = i * Self::CHUNK_SIZE;
        //                     let local_end = cmp::min(local_start + Self::CHUNK_SIZE, local_n_bytes);
        //                     self.root_process()
        //                         .gather_into(&local_vec_u8[local_start..local_end]);
        //                 }
        //             }
        //         }
        //         local_vec_u8.leak(); // discard control of the memory
        //     }
        // }
    }

    /// Root process broadcast a value f into all the processes
    #[inline]
    pub fn root_broadcast_f<F: Field>(&self, f: &mut F) {
        // unsafe {
            if self.world_size == 1 {
            } else {


                // let mut vec_u8 = Self::elem_to_u8_bytes(f, F::SIZE);
                // self.root_process().broadcast_into(&mut vec_u8);
                // vec_u8.leak();
            }
        // }
    }

    #[inline]
    /// copy the root process's memory to the buffer
    pub fn root_broadcast_bytes(&self, bytes: &mut Vec<u8>) {
        // self.root_process().broadcast_into(bytes);
        bytes.clear();
        bytes.copy_from_slice(&self.root_process());
    }

    /// sum up all local values
    #[inline]
    pub fn sum_vec<F: Field>(&mut self, local_vec: &Vec<F>) -> Vec<F> {
        if self.world_size == 1 {
            local_vec.clone()
        } else if self.world_rank == Self::ROOT_RANK {
            let mut global_vec = vec![F::ZERO; local_vec.len() * (self.world_size as usize)];
            self.gather_vec(local_vec, &mut global_vec);
            for i in 0..local_vec.len() {
                for j in 1..(self.world_size as usize) {
                    global_vec[i] = global_vec[i] + global_vec[j * local_vec.len() + i];
                }
            }
            global_vec.truncate(local_vec.len());
            global_vec
        } else {
            self.gather_vec(local_vec, &mut vec![]);
            vec![]
        }
    }

    /// coef has a length of mpi_world_size
    #[inline]
    pub fn coef_combine_vec<F: Field>(&mut self, local_vec: &Vec<F>, coef: &[F]) -> Vec<F> {
        if self.world_size == 1 {
            // Warning: literally, it should be coef[0] * local_vec
            // but coef[0] is always one in our use case of self.world_size = 1
            local_vec.clone()
        } else if self.world_rank == Self::ROOT_RANK {
            let mut global_vec = vec![F::ZERO; local_vec.len() * (self.world_size as usize)];
            let mut ret = vec![F::ZERO; local_vec.len()];
            self.gather_vec(local_vec, &mut global_vec);
            for i in 0..local_vec.len() {
                for j in 0..(self.world_size as usize) {
                    ret[i] += global_vec[j * local_vec.len() + i] * coef[j];
                }
            }
            ret
        } else {
            self.gather_vec(local_vec, &mut vec![]);
            vec![]
        }
    }

    #[inline(always)]
    pub fn world_size(&self) -> usize {
        self.world_size as usize
    }

    #[inline(always)]
    pub fn world_rank(&self) -> usize {
        self.world_rank as usize
    }

    #[inline(always)]
    pub fn is_root(&self) -> bool {
        self.world_rank == Self::ROOT_RANK
    }

    #[inline(always)]
    pub fn root_process(&self) -> &Vec<u8> {
        &self.worlds[0]

        // self.world.unwrap().process_at_rank(Self::ROOT_RANK)
    }

    #[inline(always)]
    pub fn barrier(&self) {
        // do nothing
        // self.world.unwrap().barrier();
    }
}

unsafe impl Send for MPIConfig {}
