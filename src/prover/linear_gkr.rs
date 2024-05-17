use crate::{Circuit, Config, GkrScratchpad, Proof, RawCommitment, Transcript, VectorizedM31, M31};

type FPrimitive = M31;
type F = VectorizedM31;

pub struct Prover {
    config: Config,
    sp: Vec<GkrScratchpad>,
}

impl Prover {
    pub fn new(config: &Config) -> Self {
        assert_eq!(config.field_type, crate::config::FieldType::M31);
        assert_eq!(config.fs_hash, crate::config::FiatShamirHashType::SHA256);
        assert_eq!(
            config.polynomial_commitment_type,
            crate::config::PolynomialCommitmentType::Raw
        );
        Prover {
            config: config.clone(),
            sp: Vec::new(),
        }
    }

    pub fn prepare_mem(&mut self, c: &Circuit) {
        let max_num_input_var = c
            .layers
            .iter()
            .map(|layer| layer.input_var_num)
            .max()
            .unwrap();
        let max_num_output_var = c
            .layers
            .iter()
            .map(|layer| layer.output_var_num)
            .max()
            .unwrap();
        self.sp = (0..self.config.get_num_repetitions())
            .map(|_| GkrScratchpad::new(max_num_input_var, max_num_output_var))
            .collect();
    }

    pub fn prove(&mut self, c: &Circuit) -> (Vec<F>, Proof) {
        // std::thread::sleep(std::time::Duration::from_secs(1)); // TODO

        // PC commit
        let commitment = RawCommitment::new(c.layers[0].input_vals.evals.clone());
        let buffer_v = vec![F::default(); commitment.size() / F::SIZE];
        let buffer = unsafe {
            std::slice::from_raw_parts_mut(buffer_v.as_ptr() as *mut u8, commitment.size())
        };
        commitment.serialize_into(buffer);
        let transcript = Transcript::new();
        transcript.append_u8_slice(buffer, commitment.size());

        // grinding
        todo!()
    }
}
