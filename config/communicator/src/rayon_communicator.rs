use std::sync::{mpsc, Arc, Mutex};

use crate::ExpanderComm;


#[derive(Debug, Clone)]
pub struct RayonCommunicator {
    pub world_size: usize,
    pub world_rank: usize,
    pub sender: mpsc::Sender<(usize, Vec<u8>)>,
    pub receiver: Arc<Mutex<mpsc::Receiver<(usize, Vec<u8>)>>>,
}

impl ExpanderComm for RayonCommunicator {
    fn new(world_size: usize) -> Self {
        todo!()
    }

    fn finalize() {} // nothing to do for rayon

    fn new_for_verifier(world_size: i32) -> Self {
        todo!()
    }

    fn gather_vec<F: arith::Field>(&self, local_vec: &Vec<F>, global_vec: &mut Vec<F>) {
        todo!()
    }

    fn root_broadcast_f<F: arith::Field>(&self, f: &mut F) {
        todo!()
    }

    fn root_broadcast_bytes(&self, bytes: &mut Vec<u8>) {
        todo!()
    }

    fn world_size(&self) -> usize {
        todo!()
    }

    fn world_rank(&self) -> usize {
        todo!()
    }

    fn is_root(&self) -> bool {
        todo!()
    }

    fn barrier(&self) {
        todo!()
    }
}