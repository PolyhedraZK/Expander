// use crate::Transcript;
// use arith::Field;
// use mpi_config::MPIConfig;

// /// broadcast root transcript state. incurs an additional hash if self.world_size > 1
// pub fn transcript_sync_up<F, T>(transcript: &mut T, mpi_config: &MPIConfig)
// where
//     F: Field,
//     T: Transcript<F>,
// {
//     if mpi_config.world_size > 1 {
//         let payload = MPIConfig::sync

//         let mut state = transcript.hash_and_return_state();
//         mpi_config.root_broadcast_bytes(&mut state);
//         transcript.set_state(&state);
//     }
// }
