use crate::Transcript;
use arith::Field;
use communicator::{ExpanderComm, MPICommunicator};

/// broadcast root transcript state. incurs an additional hash if self.world_size > 1
pub fn transcript_root_broadcast<F, T>(transcript: &mut T, mpi_comm: &MPICommunicator)
where
    F: Field,
    T: Transcript<F>,
{
    if mpi_comm.world_size == 1 {
    } else {
        let mut state = transcript.hash_and_return_state();
        mpi_comm.root_broadcast_bytes(&mut state);
        transcript.set_state(&state);
    }
}
