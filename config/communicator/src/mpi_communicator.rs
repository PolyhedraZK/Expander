use std::{
    cmp, env,
    ffi::{c_int, CString},
    fmt::Debug,
    process::exit,
};

use arith::Field;
use mpi::{
    ffi::{
        self, MPI_Comm_rank, MPI_Comm_size, MPI_Comm_spawn, MPI_Finalize, MPI_Init,
        RSMPI_COMM_NULL, RSMPI_COMM_WORLD, RSMPI_INFO_NULL,
    },
    topology::{Process, SimpleCommunicator},
    traits::*,
};

use crate::ExpanderComm;

static mut MPI_INITIALIZED: bool = false;
static mut WORLD: Option<SimpleCommunicator> = None;

#[derive(Clone)]
pub struct MPICommunicator {
    pub world: Option<&'static SimpleCommunicator>,
    pub world_size: i32,
    pub world_rank: i32,
}

impl Default for MPICommunicator {
    fn default() -> Self {
        Self {
            world: None,
            world_size: 1,
            world_rank: 0,
        }
    }
}

impl Debug for MPICommunicator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let world_fmt = if self.world.is_none() {
            Option::<usize>::None
        } else {
            Some(0usize)
        };

        f.debug_struct("MPICommunicator")
            .field("world", &world_fmt)
            .field("world_size", &self.world_size)
            .field("world_rank", &self.world_rank)
            .finish()
    }
}

// Note: may not be correct
impl PartialEq for MPICommunicator {
    fn eq(&self, other: &Self) -> bool {
        self.world_rank == other.world_rank && self.world_size == other.world_size
    }
}

impl MPICommunicator {
    const ROOT_RANK: i32 = 0;

    /// The communication limit for MPI is 2^30. Save 10 bits for #parties here.
    const CHUNK_SIZE: usize = 1usize << 20;

    // OK if already initialized, mpi::initialize() will return None
    #[allow(static_mut_refs)]
    unsafe fn init(world_size: usize) {
        let args: Vec<String> = env::args().collect();
        let args_vec_cstring = args
            .iter()
            .map(|arg| CString::new(arg.as_str()).unwrap())
            .collect::<Vec<_>>();
        let mut args_vec_ptr = args_vec_cstring
            .iter()
            .map(|cstring| cstring.as_ptr() as *mut i8)
            .collect::<Vec<_>>();
        let args_ptr_ptr = args_vec_ptr.as_mut_ptr();

        let mut argc: c_int = args.len() as c_int;
        let mut argv = if argc > 1 {
            args_ptr_ptr
        } else {
            std::ptr::null_mut()
        };

        MPI_Init(&mut argc, &mut argv);
        let mut rank: c_int = 0;
        let mut size: c_int = 0;
        MPI_Comm_rank(RSMPI_COMM_WORLD, &mut rank);
        MPI_Comm_size(RSMPI_COMM_WORLD, &mut size);

        // If the user explicitly run with "mpiexec" or "mpirun", we will use the world size
        // specified there, no matter what the config says
        if size > 1 {
            if world_size != size as usize {
                println!("Warning: MPI size mismatch. The config specifies {} processes, but the MPI size is {}", world_size, size);
                println!("Warning: The program will continue with MPI size = {} as specified by the user", size);
            }
            WORLD = Some(SimpleCommunicator::world());
        }

        // If the user did not run with 'mpiexec' but the config says world_size > 1, we will spawn
        // the processes for the user
        if world_size > 1 {
            let command = CString::new(args[0].clone()).unwrap();
            let mut child = RSMPI_COMM_NULL;
            const MPI_ERRCODES_IGNORE: *mut c_int = std::ptr::null_mut();
            MPI_Comm_spawn(
                command.as_ptr(),
                argv,
                world_size as c_int,
                RSMPI_INFO_NULL,
                Self::ROOT_RANK,
                RSMPI_COMM_WORLD,
                &mut child,
                MPI_ERRCODES_IGNORE,
            );
            println!("MPI spawned {} processes", world_size);
            MPI_Finalize();
            exit(0); // The parent process exits after spawning the child processes
        }
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

    #[inline(always)]
    pub fn root_process(&self) -> Process {
        self.world.unwrap().process_at_rank(Self::ROOT_RANK)
    }
}

/// MPI toolkit:
impl ExpanderComm for MPICommunicator {
    const COMMUNICATOR: crate::Communicator = crate::Communicator::MPI;

    #[inline]
    fn finalize() {
        unsafe { ffi::MPI_Finalize() };
    }

    #[allow(static_mut_refs)]
    fn new(world_size: usize) -> Self {
        unsafe {
            if !MPI_INITIALIZED {
                Self::init(world_size);
                MPI_INITIALIZED = true;
            }
        }

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
            world,
            world_size,
            world_rank,
        }
    }

    #[inline]
    fn new_for_verifier(world_size: i32) -> Self {
        Self {
            world: None,
            world_size,
            world_rank: 0,
        }
    }

    #[allow(clippy::collapsible_else_if)]
    fn gather_vec<F: Field>(&self, local_vec: &Vec<F>, global_vec: &mut Vec<F>) {
        unsafe {
            if self.world_size == 1 {
                *global_vec = local_vec.clone()
            } else {
                assert!(!self.is_root() || global_vec.len() == local_vec.len() * self.world_size());

                let local_vec_u8 = Self::vec_to_u8_bytes(local_vec);
                let local_n_bytes = local_vec_u8.len();
                let n_chunks = (local_n_bytes + Self::CHUNK_SIZE - 1) / Self::CHUNK_SIZE;
                if n_chunks == 1 {
                    if self.world_rank == Self::ROOT_RANK {
                        let mut global_vec_u8 = Self::vec_to_u8_bytes(global_vec);
                        self.root_process()
                            .gather_into_root(&local_vec_u8, &mut global_vec_u8);
                        global_vec_u8.leak(); // discard control of the memory
                    } else {
                        self.root_process().gather_into(&local_vec_u8);
                    }
                } else {
                    if self.world_rank == Self::ROOT_RANK {
                        let mut chunk_buffer_u8 = vec![0u8; Self::CHUNK_SIZE * self.world_size()];
                        let mut global_vec_u8 = Self::vec_to_u8_bytes(global_vec);
                        for i in 0..n_chunks {
                            let local_start = i * Self::CHUNK_SIZE;
                            let local_end = cmp::min(local_start + Self::CHUNK_SIZE, local_n_bytes);
                            self.root_process().gather_into_root(
                                &local_vec_u8[local_start..local_end],
                                &mut chunk_buffer_u8,
                            );

                            // distribute the data to where they belong to in global vec
                            let actual_chunk_size = local_end - local_start;
                            for j in 0..self.world_size() {
                                let global_start = j * local_n_bytes + local_start;
                                let global_end = global_start + actual_chunk_size;
                                global_vec_u8[global_start..global_end].copy_from_slice(
                                    &chunk_buffer_u8[j * Self::CHUNK_SIZE
                                        ..j * Self::CHUNK_SIZE + actual_chunk_size],
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

    /// Root process broadcase a value f into all the processes
    #[inline]
    fn root_broadcast_f<F: Field>(&self, f: &mut F) {
        unsafe {
            if self.world_size == 1 {
            } else {
                let mut vec_u8 = Self::elem_to_u8_bytes(f, F::SIZE);
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
    fn sum_vec<F: Field>(&self, local_vec: &Vec<F>) -> Vec<F> {
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
    fn coef_combine_vec<F: Field>(&self, local_vec: &Vec<F>, coef: &[F]) -> Vec<F> {
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
    fn world_size(&self) -> usize {
        self.world_size as usize
    }

    #[inline(always)]
    fn world_rank(&self) -> usize {
        self.world_rank as usize
    }

    #[inline(always)]
    fn is_root(&self) -> bool {
        self.world_rank == Self::ROOT_RANK
    }

    #[inline(always)]
    fn barrier(&self) {
        self.world.unwrap().barrier();
    }
}

unsafe impl Send for MPICommunicator {}
