use core::random;
use std::io::{Cursor, Read};

use arith::{ExtensionField, Field, SimdField};
use circuit::Circuit;
use gkr_engine::{ExpanderDualVarChallenge, ExpanderPCS, ExpanderSingleVarChallenge, FieldEngine, GKREngine, Transcript};
use serdes::ExpSerde;
use sumcheck::SUMCHECK_GKR_SIMD_MPI_DEGREE;
use transcript::RandomTape;

type Commitment<Cfg: GKREngine> = <Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::Commitment; 
type Opening<Cfg: GKREngine> = <Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::Opening;


// ================ Structured Claims ================
#[derive(Clone, Debug, Default)]
pub struct SumcheckClaim<F: FieldEngine> {
    pub challenge: ExpanderDualVarChallenge<F>,
    pub claim_x: F::ChallengeField,
    pub alpha: Option<F::ChallengeField>,
    pub claim_y: Option<F::ChallengeField>,
}

// ================ Verification Unit ================
#[derive(Clone, Debug, Default)]
pub struct SumcheckVerificationUnit<F: FieldEngine> {
    pub random_tape: RandomTape<F::ChallengeField>,
    pub claim: SumcheckClaim<F>,
    pub proof: Vec<u8>,
}


#[inline(always)]
pub fn parse_challenge_field<ChallengeF: ExtensionField>(
    mut proof_reader: impl Read,
    transcript: &mut impl Transcript<ChallengeF>,
    proof_bytes: &mut Vec<u8>,
) -> ChallengeF {
    let mut buffer = vec![0; ChallengeF::SIZE];
    proof_reader.read_exact(&mut buffer).unwrap();
    proof_bytes.extend_from_slice(&buffer);
    let challenge = ChallengeF::deserialize_from(Cursor::new(buffer)).unwrap();
    transcript.append_field_element(&challenge);
    challenge
}

pub fn parse_sumcheck_rounds<F: FieldEngine>(
    mut proof_reader: impl Read,
    n_rounds: usize,
    degree: usize,
    transcript: &mut impl Transcript<F::ChallengeField>,
    proof_bytes: &mut Vec<u8>,
    random_tape: &mut RandomTape<F::ChallengeField>,
) {
    (0..n_rounds).for_each(|_| {
        (0..degree+1).for_each(|_| {
            parse_challenge_field::<F::ChallengeField>(
                &mut proof_reader,
                transcript,
                proof_bytes,
            );
        });

        random_tape.tape.push(transcript.generate_challenge_field_element());
    });
}

pub fn parse_proof<F: FieldEngine>(
    mut proof_reader: impl Read,
    circuit: &Circuit<F>,
    proving_time_mpi_size: usize,
    xy_var_degree: usize,
    claimed_v: F::ChallengeField,
    transcript: &mut impl Transcript<F::ChallengeField>,
) -> Vec<SumcheckVerificationUnit<F>> {
    let mut verification_units = vec![SumcheckVerificationUnit::<F>::default(); circuit.layers.len()];
    let n_output_vars = circuit.layers.last().unwrap().output_var_num;
    let n_simd_vars = <F::SimdCircuitField as SimdField>::PACK_SIZE.trailing_zeros() as usize;
    let n_mpi_vars = proving_time_mpi_size.trailing_zeros() as usize;

    let mut challenge: ExpanderDualVarChallenge<F> = ExpanderSingleVarChallenge::sample_from_transcript(
        transcript,
        n_output_vars,
        proving_time_mpi_size,
    ).into();
    let mut claimed_x = claimed_v;
    let alpha = None;
    let mut claimed_y = None;

    for i in (0..circuit.layers.len()).rev() {
        let verification_unit = &mut verification_units[circuit.layers.len() - 1 - i];
        verification_unit.claim.challenge = challenge.clone();
        verification_unit.claim.claim_x = claimed_x;
        verification_unit.claim.alpha = alpha;
        verification_unit.claim.claim_y = claimed_y;

        let layer = &circuit.layers[i];
        let sumcheck_proof = &mut verification_unit.proof;
        let random_tape = &mut verification_unit.random_tape;
        let n_vars = layer.input_var_num;
        
        parse_sumcheck_rounds::<F>(
            &mut proof_reader,
            n_vars,
            xy_var_degree,
            transcript,
            sumcheck_proof,
            random_tape,
        );

        parse_sumcheck_rounds::<F>(
            &mut proof_reader,
            n_simd_vars,
            SUMCHECK_GKR_SIMD_MPI_DEGREE,
            transcript,
            sumcheck_proof,
            random_tape,
        );
        
        parse_sumcheck_rounds::<F>(
            &mut proof_reader,
            n_mpi_vars,
            SUMCHECK_GKR_SIMD_MPI_DEGREE,
            transcript,
            sumcheck_proof,
            random_tape,
        );

        challenge.rz_0 = parse_challenge_field::<F::ChallengeField>(
            &mut proof_reader,
            transcript,
            sumcheck_proof,
        );

        if !layer.structure_info.skip_sumcheck_phase_two {
            parse_sumcheck_rounds::<F>(
                &mut proof_reader,
                n_vars,
                xy_var_degree,
                transcript,
                sumcheck_proof,
                random_tape,
            );
        }
        
    }


    unimplemented!()
}