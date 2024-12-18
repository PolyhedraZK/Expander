use crate::Transcript;
use arith::{ExtensionField, Field};
use mpi_config::MPIConfig;

/// broadcast root transcript state. incurs an additional hash if self.world_size > 1
pub fn transcript_root_broadcast<BaseF, ChallengeF, T>(transcript: &mut T, mpi_config: &MPIConfig)
where
    BaseF: Field,
    ChallengeF: ExtensionField<BaseField = BaseF>,
    T: Transcript<BaseF, ChallengeF>,
{
    if mpi_config.world_size > 1 {
        let mut state = transcript.hash_and_return_state();
        mpi_config.root_broadcast_bytes(&mut state);
        transcript.set_state(&state);
    }
}
