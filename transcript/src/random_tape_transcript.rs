use arith::ExtensionField;
use gkr_engine::Transcript;

/// A transcript that uses a random tape to generate challenges.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct RandomTape<ChallengeF: ExtensionField> {
    /// The random tape used to generate the transcript.
    pub tape: Vec<ChallengeF>,
    /// The current position in the random tape.
    pub position: usize,
}

impl<ChallengeF: ExtensionField> RandomTape<ChallengeF> {
    /// Creates a new `RandomTape` with the given random tape.
    pub fn new_with_tape(tape: Vec<ChallengeF>) -> Self {
        Self { tape, position: 0 }
    }
}

impl<ChallengeF: ExtensionField> Transcript<ChallengeF> for RandomTape<ChallengeF> {
    fn new() -> Self {
        Self {
            tape: vec![],
            position: 0,
        }
    }

    fn append_commitment(&mut self, _commitment_bytes: &[u8]) {}

    fn append_field_element(&mut self, _f: &ChallengeF) {}

    fn append_u8_slice(&mut self, _buffer: &[u8]) {}

    fn generate_circuit_field_element(&mut self) -> <ChallengeF as ExtensionField>::BaseField {
        unimplemented!()
    }

    fn generate_challenge_field_element(&mut self) -> ChallengeF {
        let challenge = self.tape[self.position];
        self.position += 1;
        challenge
    }

    fn generate_challenge_u8_slice(&mut self, _n_bytes: usize) -> Vec<u8> {
        unimplemented!()
    }

    fn finalize_and_get_proof(&self) -> gkr_engine::Proof {
        unimplemented!()
    }

    fn hash_and_return_state(&mut self) -> Vec<u8> {
        unimplemented!()
    }

    fn set_state(&mut self, _state: &[u8]) {
        unimplemented!()
    }

    fn lock_proof(&mut self) {
        unimplemented!()
    }

    fn unlock_proof(&mut self) {
        unimplemented!()
    }
}
