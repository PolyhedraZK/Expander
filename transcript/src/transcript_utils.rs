use crate::Transcript;
use arith::ExtensionField;
use mpi_config::MPIConfig;

/// broadcast root transcript state. incurs an additional hash if self.world_size > 1
pub fn transcript_root_broadcast<F, T>(transcript: &mut T, mpi_config: &MPIConfig)
where
    F: ExtensionField,
    T: Transcript<F>,
{
    if mpi_config.world_size > 1 {
        let mut state = transcript.hash_and_return_state();
        mpi_config.root_broadcast_bytes(&mut state);
        transcript.set_state(&state);
    }
}

/// Correspondence to 'transcript_root_broadcast' from the verifier side.
///
/// Note: Currently, the verifier is assumed to run on a single core with no mpi sync,
/// the word 'sync' here refers to the verifier syncing up with the prover's transcript state,
/// which is updated by 'transcript_root_broadcast' if mpi_size > 1.
pub fn transcript_verifier_sync<F, T>(transcript: &mut T, mpi_config: &MPIConfig)
where
    F: ExtensionField,
    T: Transcript<F>,
{
    if mpi_config.world_size() > 1 {
        let state = transcript.hash_and_return_state(); // Sync up the Fiat-Shamir randomness
        transcript.set_state(&state);
    }
}
