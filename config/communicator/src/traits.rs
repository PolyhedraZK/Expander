

use std::fmt::Debug;

use arith::Field;

pub enum Communicator {
    MPI,
    Rayon,
}

pub trait ExpanderComm: Clone + Debug {
    const COMMUNICATOR: Communicator;

    fn new(world_size: usize) -> Self;

    fn finalize();

    // Create a new communicator for the verifier. 
    fn new_for_verifier(world_size: i32) -> Self;

    /// Gather all local values to the root process
    #[allow(clippy::ptr_arg)] // must be vector here
    fn gather_vec<F: Field>(&self, local_vec: &Vec<F>, global_vec: &mut Vec<F>);

    /// Root process broadcase a value f into all the processes
    fn root_broadcast_f<F: Field>(&self, f: &mut F);

    /// Root process broadcase a vector of bytes into all the processes
    fn root_broadcast_bytes(&self, bytes: &mut Vec<u8>);

    /// sum up all local values
    fn sum_vec<F: Field>(&self, local_vec: &Vec<F>) -> Vec<F> {
        if self.world_size() == 1 {
            local_vec.clone()
        } else if self.is_root() {
            let mut global_vec = vec![F::ZERO; local_vec.len() * self.world_size()];
            self.gather_vec(local_vec, &mut global_vec);
            for i in 0..local_vec.len() {
                for j in 1..self.world_size() {
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
    fn coef_combine_vec<F: Field>(&self, local_vec: &Vec<F>, coef: &[F]) -> Vec<F> {
        if self.world_size() == 1 {
            // Warning: literally, it should be coef[0] * local_vec
            // but coef[0] is always one in our use case of self.world_size = 1
            local_vec.clone()
        } else if self.is_root() {
            let mut global_vec = vec![F::ZERO; local_vec.len() * self.world_size()];
            let mut ret = vec![F::ZERO; local_vec.len()];
            self.gather_vec(local_vec, &mut global_vec);
            for i in 0..local_vec.len() {
                for j in 0..self.world_size() {
                    ret[i] += global_vec[j * local_vec.len() + i] * coef[j];
                }
            }
            ret
        } else {
            self.gather_vec(local_vec, &mut vec![]);
            vec![]
        }
    }

    fn world_size(&self) -> usize;

    fn world_rank(&self) -> usize;

    fn is_root(&self) -> bool;

    fn barrier(&self);
}