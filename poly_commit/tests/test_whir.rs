use arith::{ExtensionField, Field};
use ark_std::test_rng;
use gkr_engine::{StructuredReferenceString, Transcript};
use gkr_hashers::Keccak256hasher;
use goldilocks::{Goldilocks, GoldilocksExt2};
use poly_commit::{PolynomialCommitmentScheme, WhirPCS};
use polynomials::MultiLinearPoly;
use rand::thread_rng;
use spongefish_pow::keccak::KeccakPoW;
use transcript::BytesHashTranscript;
use whir::{
    crypto::{
        fields::Field64_2,
        merkle_tree::{
            keccak::{KeccakCompress, KeccakLeafHash, KeccakMerkleTreeParams},
            parameters::default_config,
        },
    },
    parameters::{FoldingFactor, MultivariateParameters, ProtocolParameters, SoundnessType},
    whir::parameters::WhirConfig,
};

fn test_whir_pcs_helper<F: ExtensionField, T: Transcript, P: PolynomialCommitmentScheme<F>>(
    params: &P::Params,
    poly: &P::Poly,
    x: &P::EvalPoint,
) {
    let mut rng = thread_rng();
    // NOTE(HS) we assume that the polynomials we pass in are of sufficient length.
    let (srs, _) = P::gen_srs_for_testing(params, &mut rng);
    let (proving_key, verification_key) = srs.into_keys();
    let mut transcript = T::new();
    let mut scratch_pad = P::init_scratch_pad(params);

    let commitment = P::commit(params, &proving_key, poly, &mut scratch_pad);

    let mut transcript_cloned = transcript.clone();
    let (v, opening) = P::open(
        params,
        &commitment,
        &proving_key,
        poly,
        x,
        &mut scratch_pad,
        &mut transcript,
    );
    assert!(P::verify(
        params,
        &verification_key,
        &commitment,
        x,
        v,
        &opening,
        &mut transcript_cloned
    ));
}

fn test_whir_pcs(num_vars_start: usize, num_vars_end: usize) {
    let mut rng = test_rng();

    (num_vars_start..=num_vars_end).for_each(|num_vars| {
        let point = (0..num_vars)
            .map(|_| GoldilocksExt2::random_unsafe(&mut rng))
            .collect();
        let poly = MultiLinearPoly::<Goldilocks>::random(num_vars, &mut rng);

        let (leaf_hash_params, two_to_one_params) =
            default_config::<Field64_2, KeccakLeafHash<Field64_2>, KeccakCompress>(&mut rng);

        let mv_params = MultivariateParameters::<Field64_2>::new(num_vars);

        let whir_params = ProtocolParameters::<KeccakMerkleTreeParams<Field64_2>, KeccakPoW> {
            initial_statement: true,
            security_level: 100,
            pow_bits: 0,
            folding_factor: FoldingFactor::ConstantFromSecondRound(4, 2),
            leaf_hash_params,
            two_to_one_params,
            soundness_type: SoundnessType::ConjectureList,
            _pow_parameters: Default::default(),
            starting_log_inv_rate: 1,
        };

        let params = WhirConfig::<Field64_2, KeccakMerkleTreeParams<Field64_2>, KeccakPoW>::new(
            mv_params,
            whir_params,
        );

        test_whir_pcs_helper::<GoldilocksExt2, BytesHashTranscript<Keccak256hasher>, WhirPCS>(
            &params, &poly, &point,
        );
    })
}

#[test]
fn test_whir_pcs_full_e2e() {
    test_whir_pcs(10, 15)
}
