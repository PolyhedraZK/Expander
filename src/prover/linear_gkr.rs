//! This module implements the whole GKR prover, including the IOP and PCS.

use ark_std::{end_timer, start_timer};

use crate::{
    gkr_prove, gkr_square_prove, Circuit, GKRConfig, GkrScratchpad, Proof, RawCommitment,
    Transcript,
};

#[cfg(feature = "grinding")]
pub(crate) fn grind<C: GKRConfig>(transcript: &mut Transcript) {
    use arith::{Field, FieldSerde};

    let timer = start_timer!(|| format!("grind {} bits", C::GRINDING_BITS));

    let mut hash_bytes = vec![];

    // ceil(32/field_size)
    let num_field_elements = (31 + C::ChallengeField::SIZE) / C::ChallengeField::SIZE;

    let initial_hash = transcript.challenge_fs::<C>(num_field_elements);
    initial_hash
        .iter()
        .for_each(|h| h.serialize_into(&mut hash_bytes));

    assert!(hash_bytes.len() >= 32, "hash len: {}", hash_bytes.len());
    hash_bytes.truncate(32);

    for _ in 0..(1 << C::GRINDING_BITS) {
        transcript.hasher.hash_inplace(&mut hash_bytes, 32);
    }
    transcript.append_u8_slice(&hash_bytes[..32]);
    end_timer!(timer);
}

pub struct Prover<C: GKRConfig> {
    sp: GkrScratchpad<C>,
}

impl<C: GKRConfig> Default for Prover<C> {
    fn default() -> Self {
        Self {
            sp: GkrScratchpad::default(),
        }
    }
}

impl<C: GKRConfig> Prover<C> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prepare_mem(&mut self, c: &Circuit<C>) {
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
        self.sp = GkrScratchpad::<C>::new(max_num_input_var, max_num_output_var);
    }

    pub fn prove(&mut self, c: &Circuit<C>) -> (C::Field, Proof) {
        let timer = start_timer!(|| "prove");
        // std::thread::sleep(std::time::Duration::from_secs(1)); // TODO

        // PC commit
        let commitment = RawCommitment::<C>::new(c.layers[0].input_vals.evals.clone());

        let mut buffer = vec![];
        commitment.serialize_into(&mut buffer);
        let mut transcript = Transcript::new();
        transcript.append_u8_slice(&buffer);

        #[cfg(feature = "grinding")]
        grind::<C>(&mut transcript);

        let claimed_v: C::Field;
        let mut _rz0s = vec![];
        let mut _rz1s = vec![];

        if C::GKR_SQUARE {
            (claimed_v, _rz0s) = gkr_square_prove(c, &mut self.sp, &mut transcript);
        } else {
            (claimed_v, _rz0s, _rz1s) = gkr_prove(c, &mut self.sp, &mut transcript);
        }

        // open
        match C::POLYNOMIAL_COMMITMENT_TYPE {
            crate::config::PolynomialCommitmentType::Raw => {
                // no need to update transcript
            }
            _ => todo!(),
        }
        end_timer!(timer);
        (claimed_v, transcript.proof)
    }
}
