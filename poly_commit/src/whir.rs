use arith::FFTField;
use ark_std::log2;
use gkr_engine::{StructuredReferenceString, Transcript};
use goldilocks::{Goldilocks, GoldilocksExt2};
use polynomials::MultiLinearPoly;
use spongefish::{DomainSeparator, ProverState};
use spongefish_pow::keccak::KeccakPoW;
use whir::{
    crypto::{
        fields::{Field64, Field64_2},
        merkle_tree::keccak::{KeccakCompress, KeccakMerkleTreeParams},
    },
    poly_utils::{coeffs::CoefficientList, evals::EvaluationsList, multilinear::MultilinearPoint},
    whir::{
        committer::{CommitmentReader, CommitmentWriter},
        domainsep::WhirDomainSeparator,
        parameters::WhirConfig,
        prover::Prover,
        statement::{Statement, Weights},
        verifier::Verifier,
    },
};

use crate::PolynomialCommitmentScheme;

/// Whir Polynomial Commitment Scheme over Goldilocks field.
// Note: Hard coded to Goldilocks field.
pub struct WhirPCS;

pub type WhirCommitment =
    whir::whir::committer::Witness<Field64_2, KeccakMerkleTreeParams<Field64_2>>;
pub type WhirParam = WhirConfig<Field64_2, KeccakMerkleTreeParams<Field64_2>, KeccakPoW>;

impl PolynomialCommitmentScheme<GoldilocksExt2> for WhirPCS {
    const NAME: &'static str = "WhirPCS";

    type Params = WhirParam;
    type Poly = MultiLinearPoly<Goldilocks>;
    type EvalPoint = Vec<GoldilocksExt2>;
    type ScratchPad = ProverState;

    type SRS = ();
    type Commitment = WhirCommitment; //Vec<u8>; //WhirCommitment;
    type Opening = Vec<u8>; //Goldilocks;

    fn init_scratch_pad(params: &Self::Params) -> Self::ScratchPad {
        // todo: session identifier can be sampled from transcript?
        let domainsep = DomainSeparator::new("🌪️")
            .commit_statement(&params)
            .add_whir_proof(&params);

        domainsep.to_prover_state()
    }

    fn gen_srs_for_testing(_params: &Self::Params, _rng: impl rand::RngCore) -> (Self::SRS, usize) {
        ((), 0)
    }

    fn commit(
        params: &Self::Params,
        _proving_key: &Self::SRS,
        poly: &Self::Poly,
        prover_state: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        let whir_poly = CoefficientList::new(
            poly.coeffs
                .iter()
                .map(|&coeff| Field64::from(coeff))
                .collect::<Vec<_>>(),
        );

        let committer = CommitmentWriter::new(params.clone());

        let witness = committer.commit(prover_state, whir_poly.clone()).unwrap();

        witness
    }

    fn open(
        params: &Self::Params,
        commitment: &Self::Commitment,
        proving_key: &<Self::SRS as gkr_engine::StructuredReferenceString>::PKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        prover_state: &mut Self::ScratchPad,
        _transcript: &mut impl gkr_engine::Transcript,
    ) -> (GoldilocksExt2, Self::Opening) {
        // todo: avoid cloning commitment if possible
        let witness = (*commitment).clone();

        let num_variables = log2(poly.coeffs.len()) as usize;
        let mut statement = Statement::<Field64_2>::new(num_variables);

        let whir_poly = CoefficientList::new(
            poly.coeffs
                .iter()
                .map(|&coeff| Field64::from(coeff))
                .collect::<Vec<_>>(),
        );

        let point = MultilinearPoint(
            x.iter()
                .map(|&coord| Field64_2::from(coord))
                .collect::<Vec<_>>(),
        );

        let eval = whir_poly.evaluate_at_extension(&point);
        let weights = Weights::evaluation(point.clone());
        statement.add_constraint(weights, eval);

        let prover = Prover(params.clone());
        prover
            .prove(prover_state, statement.clone(), witness)
            .unwrap();

        (eval.into(), prover_state.narg_string().to_vec())
    }

    fn verify(
        params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        eval: GoldilocksExt2,
        opening: &Self::Opening,
        transcript: &mut impl Transcript,
    ) -> bool {
        let num_variables = x.len();
        let mut statement = Statement::<Field64_2>::new(num_variables);

        let point = MultilinearPoint(
            x.iter()
                .map(|&coord| Field64_2::from(coord))
                .collect::<Vec<_>>(),
        );
        let weights = Weights::evaluation(point.clone());
        statement.add_constraint(weights, eval.into());

        let commitment_reader = CommitmentReader::new(&params);
        let verifier = Verifier::new(&params);

        let domainsep = DomainSeparator::new("🌪️")
            .commit_statement(&params)
            .add_whir_proof(&params);

        let mut verifier_state = domainsep.to_verifier_state(opening);
        let parsed_commitment = commitment_reader
            .parse_commitment(&mut verifier_state)
            .unwrap();

        let (point, constraint) =
            match verifier.verify(&mut verifier_state, &parsed_commitment, &statement) {
                Ok((p, c)) => (p, c),
                Err(_) => return false,
            };

        println!("WhirPCS: Verifying point: {:?}", point);
        println!("WhirPCS: Verifying constraint: {:?}", constraint);

        true
    }
}
