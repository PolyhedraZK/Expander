use crate::{Circuit, Config, GkrScratchpad, Transcript, VectorizedM31, M31};

type FPrimitive = M31;
type F = VectorizedM31;

pub fn gkr_prove(
    circuit: &Circuit,
    gkr_scratchpad: &mut [GkrScratchpad],
    transcript: &mut Transcript,
    config: &Config,
) -> (Vec<F>, Vec<FPrimitive>, Vec<FPrimitive>) {
    // Prove the circuit using the GKR protocol
    todo!()
}
