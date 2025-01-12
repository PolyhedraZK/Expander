use crate::Transcript;
use arith::Field;
use mpi_config::MPIConfig;

/// broadcast root transcript state. incurs an additional hash if self.world_size > 1
pub fn transcript_root_broadcast<F, T>(transcript: &mut T, mpi_config: &MPIConfig)
where
    F: Field,
    T: Transcript<F>,
{
    if mpi_config.world_size > 1 {
        let mut state = transcript.hash_and_return_state();
        mpi_config.root_broadcast_bytes(&mut state);
        transcript.set_state(&state);
    }
}

/// sync verifier transcript state. incurs an additional hash if self.world_size > 1
/// corresponding part to the transcript_root_broadcast on verifier side
pub fn transcript_verifier_sync<F, T>(transcript: &mut T, mpi_config: &MPIConfig)
where
    F: Field,
    T: Transcript<F>,
{
    if mpi_config.world_size() > 1 {
        let state = transcript.hash_and_return_state(); // Sync up the Fiat-Shamir randomness
        transcript.set_state(&state);
    }
}
