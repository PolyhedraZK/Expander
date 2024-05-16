use crate::{Circuit, Config};

pub struct Prover {}

impl Prover {
    pub fn new(config: &Config) -> Self {
        Prover {}
    }
    pub fn prepare_mem(&mut self, c: &Circuit) {}
    pub fn prove(&mut self, c: &Circuit) {
        std::thread::sleep(std::time::Duration::from_secs(1)); // TODO
    }
}
