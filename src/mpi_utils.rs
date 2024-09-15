use arith::Field;
use mpi::{
    topology::{Process, SimpleCommunicator},
    traits::*,
};

use crate::{FiatShamirHash, Transcript};

pub static mut MPI_UNIVERSE: Option<mpi::environment::Universe> = None;
pub static mut MPI_WORLD: Option<SimpleCommunicator> = None;
pub static mut MPI_SIZE: i32 = 1;
pub static mut MPI_RANK: i32 = 0;

pub static MPI_ROOT_RANK: i32 = 0;
pub static mut MPI_ROOT_PROCESS: Option<Process> = None;

pub fn mpi_init() {
    unsafe {
        MPI_UNIVERSE = mpi::initialize();
        MPI_WORLD = Some(MPI_UNIVERSE.as_ref().unwrap().world());
        MPI_SIZE = MPI_WORLD.as_ref().unwrap().size();
        MPI_RANK = MPI_WORLD.as_ref().unwrap().rank();

        MPI_ROOT_PROCESS = Some(MPI_WORLD.as_ref().unwrap().process_at_rank(MPI_ROOT_RANK));
    }
}

#[macro_export]
macro_rules! root_println {
    () => {println!();};
    ($($arg:tt)*) => {
        if MPIToolKit::is_root() {
            println!($($arg)*);
        }
    };
}

pub struct MPIToolKit {}

/// MPI toolkit:
/// Note: if mpi_init is never called, the tools should work normally as if mpi_size == 1
impl MPIToolKit {
    /// Return an u8 vector sharing THE SAME MEMORY SLOT with the input.
    unsafe fn elem_to_u8_bytes<V: Sized>(elem: &V, byte_size: usize) -> Vec<u8> {
        Vec::<u8>::from_raw_parts((elem as *const V) as *mut u8, byte_size, byte_size)
    }

    /// Return an u8 vector sharing THE SAME MEMORY SLOT with the input.
    unsafe fn vec_to_u8_bytes<F: Field>(vec: &Vec<F>) -> Vec<u8> {
        Vec::<u8>::from_raw_parts(
            vec.as_ptr() as *mut u8,
            vec.len() * F::SIZE,
            vec.capacity() * F::SIZE,
        )
    }

    pub fn gather_vec<F: Field>(local_vec: &Vec<F>, global_vec: &mut Vec<F>) {
        unsafe {
            debug_assert!(global_vec.len() >= local_vec.len() * (MPI_SIZE as usize));
            if MPI_SIZE == 1 {
                *global_vec = local_vec.clone()
            } else if MPI_RANK == MPI_ROOT_RANK {
                let local_vec_u8 = Self::vec_to_u8_bytes(local_vec);
                let mut global_vec_u8 = Self::vec_to_u8_bytes(global_vec);
                MPI_ROOT_PROCESS
                    .unwrap()
                    .gather_into_root(&local_vec_u8, &mut global_vec_u8);
                local_vec_u8.leak(); // discard control of the memory
                global_vec_u8.leak();
            } else {
                let local_vec_u8 = Self::vec_to_u8_bytes(local_vec);
                MPI_ROOT_PROCESS.unwrap().gather_into(&local_vec_u8);
                local_vec_u8.leak();
            }
        }
    }

    /// broadcast root transcript state. incurs an additional hash if MPI_SIZE > 1
    pub fn transcript_sync_up<H: FiatShamirHash>(transcript: &mut Transcript<H>) {
        unsafe {
            if MPI_SIZE == 1 {
            } else {
                transcript.hash_to_digest();
                MPI_ROOT_PROCESS
                    .unwrap()
                    .broadcast_into(&mut transcript.digest);
            }
        }
    }

    /// Root process broadcase a value f into all the processes
    pub fn root_broadcast<F: Field>(f: &F) {
        unsafe {
            if MPI_SIZE == 1 {
            } else {
                let mut vec_u8 = Self::elem_to_u8_bytes(f, F::SIZE);
                MPI_ROOT_PROCESS.unwrap().broadcast_into(&mut vec_u8);
                vec_u8.leak();
            }
        }
    }

    ///
    pub fn sum_vec<F: Field>(local_vec: &Vec<F>) -> Vec<F> {
        unsafe {
            if MPI_SIZE == 1 {
                local_vec.clone()
            } else if MPI_RANK == MPI_ROOT_RANK {
                let mut global_vec = vec![F::ZERO; local_vec.len() * (MPI_SIZE as usize)];
                Self::gather_vec(local_vec, &mut global_vec);
                for i in 0..local_vec.len() {
                    for j in 1..(MPI_SIZE as usize) {
                        global_vec[i] = global_vec[i] + global_vec[j * local_vec.len() + i];
                    }
                }
                global_vec.truncate(local_vec.len());
                global_vec
            } else {
                Self::gather_vec(local_vec, &mut vec![]);
                vec![]
            }
        }
    }

    ///
    pub fn coef_combine_vec<F: Field>(local_vec: &Vec<F>, coef: &Vec<F>) -> Vec<F> {
        unsafe {
            if MPI_SIZE == 1 {
                // Warning: literally, it should be coef[0] * local_vec
                // but coef[0] is always one in our use case of MPI_SIZE = 1
                local_vec.clone()
            } else if MPI_RANK == MPI_ROOT_RANK {
                let mut global_vec = vec![F::ZERO; local_vec.len() * (MPI_SIZE as usize)];
                let mut ret = vec![F::ZERO; local_vec.len()];
                Self::gather_vec(local_vec, &mut global_vec);
                for i in 0..local_vec.len() {
                    for j in 0..(MPI_SIZE as usize) {
                        ret[i] = ret[i] + global_vec[j * local_vec.len() + i] * coef[j];
                    }
                }
                ret
            } else {
                Self::gather_vec(local_vec, &mut vec![]);
                vec![]
            }
        }
    }

    #[inline(always)]
    pub fn world_size() -> usize {
        unsafe { MPI_SIZE as usize }
    }

    #[inline(always)]
    pub fn world_rank() -> usize {
        unsafe { MPI_RANK as usize }
    }

    #[inline(always)]
    pub fn is_root() -> bool {
        unsafe { MPI_RANK == MPI_ROOT_RANK }
    }
}
