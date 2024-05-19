use crate::{
    gkr_prove, scratchpad, Circuit, Config, GkrScratchpad, Proof, RawCommitment, Transcript,
    VectorizedM31, M31,
};

type FPrimitive = M31;
type F = VectorizedM31;

fn grind(transcript: &mut Transcript, config: &Config) {
    let initial_hash = transcript.challenge_fs(256 / config.field_size);
    let mut hash_bytes = [0u8; 256 / 8];
    let mut offset = 0;
    for i in 0..initial_hash.len() {
        initial_hash[i].serialize_into(&mut hash_bytes[offset..]);
        offset += (config.field_size + 7) / 8;
    }
    for i in 0..(1 << config.grinding_bits) {
        transcript.hasher.hash_inplace(&mut hash_bytes, 256 / 8);
    }
    transcript.append_u8_slice(&hash_bytes, 256 / 8);
}

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
        let mut transcript = Transcript::new();
        transcript.append_u8_slice(buffer, commitment.size());

        grind(&mut transcript, &self.config);

        let (claimed_v, rz0s, rz1s) = gkr_prove(c, &mut self.sp, &mut transcript, &self.config);

        // open
        match self.config.polynomial_commitment_type {
            crate::config::PolynomialCommitmentType::Raw => {
                // no need to update transcript
            }
            _ => todo!(),
        }
        (claimed_v, transcript.proof)
    }
}
