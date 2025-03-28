use std::{cmp, fmt::Debug};

use arith::Field;
use mpi::{
    environment::Universe,
    ffi,
    topology::{Process, SimpleCommunicator},
    traits::*,
};

use super::MPIEngine;

#[macro_export]
macro_rules! root_println {
    ($config: expr, $($arg:tt)*) => {
        if $config.is_root() {
            println!($($arg)*);
        }
    };
}

static mut UNIVERSE: Option<Universe> = None;
static mut WORLD: Option<SimpleCommunicator> = None;

#[derive(Clone)]
pub struct MPIConfig {
    pub universe: Option<&'static mpi::environment::Universe>,
    pub world: Option<&'static SimpleCommunicator>,
    pub world_size: i32,
    pub world_rank: i32,
}

impl Default for MPIConfig {
    fn default() -> Self {
        Self {
            universe: None,
            world: None,
            world_size: 1,
            world_rank: 0,
        }
    }
}

impl Debug for MPIConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let universe_fmt = if self.universe.is_none() {
            Option::<usize>::None
        } else {
            Some(self.universe.unwrap().buffer_size())
        };

        let world_fmt = if self.world.is_none() {
            Option::<usize>::None
        } else {
            Some(0usize)
        };

        f.debug_struct("MPIConfig")
            .field("universe", &universe_fmt)
            .field("world", &world_fmt)
            .field("world_size", &self.world_size)
            .field("world_rank", &self.world_rank)
            .finish()
    }
}

// Note: may not be correct
impl PartialEq for MPIConfig {
    fn eq(&self, other: &Self) -> bool {
        self.world_rank == other.world_rank && self.world_size == other.world_size
    }
}

/// MPI toolkit:
impl MPIEngine for MPIConfig {
    const ROOT_RANK: i32 = 0;

    /// The communication limit for MPI is 2^30. Save 10 bits for #parties here.
    const CHUNK_SIZE: usize = 1usize << 20;

    // OK if already initialized, mpi::initialize() will return None
    #[allow(static_mut_refs)]
    fn init() {
        unsafe {
            let universe = mpi::initialize();
            if universe.is_some() {
                UNIVERSE = universe;
                WORLD = Some(UNIVERSE.as_ref().unwrap().world());
            }
        }
    }

    #[inline]
    fn finalize() {
        unsafe { ffi::MPI_Finalize() };
    }

    #[allow(static_mut_refs)]
    fn prover_new() -> Self {
        Self::init();
        let universe = unsafe { UNIVERSE.as_ref() };
        let world = unsafe { WORLD.as_ref() };
        let world_size = if let Some(world) = world {
            world.size()
        } else {
            1
        };
        let world_rank = if let Some(world) = world {
            world.rank()
        } else {
            0
        };
        Self {
            universe,
            world,
            world_size,
            world_rank,
        }
    }

    #[inline]
    fn verifier_new(world_size: i32) -> Self {
        Self {
            universe: None,
            world: None,
            world_size,
            world_rank: 0,
        }
    }

    #[allow(clippy::collapsible_else_if)]
    fn gather_vec<F: Sized + Clone>(&self, local_vec: &[F], global_vec: &mut Vec<F>) {
        unsafe {
            if self.world_size == 1 {
                *global_vec = local_vec.to_vec()
            } else {
                assert!(!self.is_root() || global_vec.len() == local_vec.len() * self.world_size());

                let local_vec_u8 = transmute_vec_to_u8_bytes(local_vec);
                let local_n_bytes = local_vec_u8.len();
                let n_chunks = (local_n_bytes + Self::CHUNK_SIZE - 1) / Self::CHUNK_SIZE;
                if n_chunks == 1 {
                    if self.world_rank == Self::ROOT_RANK {
                        let mut global_vec_u8 = transmute_vec_to_u8_bytes(global_vec);
                        self.root_process()
                            .gather_into_root(&local_vec_u8, &mut global_vec_u8);
                        global_vec_u8.leak(); // discard control of the memory
                    } else {
                        self.root_process().gather_into(&local_vec_u8);
                    }
                } else {
                    if self.world_rank == Self::ROOT_RANK {
                        let mut chunk_buffer_u8 = vec![0u8; Self::CHUNK_SIZE * self.world_size()];
                        let mut global_vec_u8 = transmute_vec_to_u8_bytes(global_vec);
                        for i in 0..n_chunks {
                            let local_start = i * Self::CHUNK_SIZE;
                            let local_end = cmp::min(local_start + Self::CHUNK_SIZE, local_n_bytes);
                            let actual_chunk_size = local_end - local_start;
                            if actual_chunk_size < Self::CHUNK_SIZE {
                                chunk_buffer_u8.resize(actual_chunk_size * self.world_size(), 0u8);
                            }

                            self.root_process().gather_into_root(
                                &local_vec_u8[local_start..local_end],
                                &mut chunk_buffer_u8,
                            );

                            // distribute the data to where they belong to in global vec
                            for j in 0..self.world_size() {
                                let global_start = j * local_n_bytes + local_start;
                                let global_end = global_start + actual_chunk_size;
                                global_vec_u8[global_start..global_end].copy_from_slice(
                                    &chunk_buffer_u8
                                        [j * actual_chunk_size..(j + 1) * actual_chunk_size],
                                );
                            }
                        }
                        global_vec_u8.leak(); // discard control of the memory
                    } else {
                        for i in 0..n_chunks {
                            let local_start = i * Self::CHUNK_SIZE;
                            let local_end = cmp::min(local_start + Self::CHUNK_SIZE, local_n_bytes);
                            self.root_process()
                                .gather_into(&local_vec_u8[local_start..local_end]);
                        }
                    }
                }
                local_vec_u8.leak(); // discard control of the memory
            }
        }
    }

    /// Root process broadcast a value f into all the processes
    #[inline]
    fn root_broadcast_f<F: Field>(&self, f: &mut F) {
        unsafe {
            if self.world_size == 1 {
            } else {
                let mut vec_u8 = transmute_elem_to_u8_bytes(f, F::SIZE);
                self.root_process().broadcast_into(&mut vec_u8);
                vec_u8.leak();
            }
        }
    }

    #[inline]
    fn root_broadcast_bytes(&self, bytes: &mut Vec<u8>) {
        self.root_process().broadcast_into(bytes);
    }

    /// sum up all local values
    #[inline]
    fn sum_vec<F: Field>(&self, local_vec: &[F]) -> Vec<F> {
        if self.world_size == 1 {
            local_vec.to_vec()
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
    fn coef_combine_vec<F: Field>(&self, local_vec: &[F], coef: &[F]) -> Vec<F> {
        if self.world_size == 1 {
            // Warning: literally, it should be coef[0] * local_vec
            // but coef[0] is always one in our use case of self.world_size = 1
            local_vec.to_vec()
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
            vec![F::ZERO; local_vec.len()]
        }
    }

    #[inline(always)]
    fn is_single_process(&self) -> bool {
        self.world_size == 1
    }

    #[inline(always)]
    fn world_size(&self) -> usize {
        self.world_size as usize
    }

    #[inline(always)]
    fn world_rank(&self) -> usize {
        self.world_rank as usize
    }

    #[inline(always)]
    fn root_process(&self) -> Process {
        self.world.unwrap().process_at_rank(Self::ROOT_RANK)
    }

    #[inline(always)]
    fn barrier(&self) {
        self.world.unwrap().barrier();
    }
}

unsafe impl Send for MPIConfig {}

/// Return an u8 vector sharing THE SAME MEMORY SLOT with the input.
#[inline]
unsafe fn transmute_elem_to_u8_bytes<V: Sized>(elem: &V, byte_size: usize) -> Vec<u8> {
    Vec::<u8>::from_raw_parts((elem as *const V) as *mut u8, byte_size, byte_size)
}

/// Return an u8 vector sharing THE SAME MEMORY SLOT with the input.
#[inline]
unsafe fn transmute_vec_to_u8_bytes<F: Sized>(vec: &[F]) -> Vec<u8> {
    Vec::<u8>::from_raw_parts(
        vec.as_ptr() as *mut u8,
        std::mem::size_of_val(vec),
        std::mem::size_of_val(vec),
    )
}
