use arith::BN254Fr;
use halo2curves::bn256::{G1Affine, G1};
use itertools::izip;
use polynomials::{EqPolynomial, MultiLinearPoly, MultilinearExtension};
use transcript::Transcript;

use crate::hyrax::{
    inner_prod_argument::{pedersen_ipa_prove, pedersen_ipa_verify},
    pedersen::{pedersen_setup, pedersen_vector_commit},
    PedersenIPAProof, PedersenParams,
};

#[allow(unused)]
pub(crate) fn hyrax_setup(local_vars: usize, mut rng: impl rand::RngCore) -> PedersenParams {
    let pedersen_length = (local_vars + 1) / 2;

    pedersen_setup(pedersen_length, rng)
}

pub struct HyraxCommitment(pub Vec<G1Affine>);

#[allow(unused)]
pub(crate) fn hyrax_commit(
    params: &PedersenParams,
    mle_poly: impl MultilinearExtension<BN254Fr>,
) -> HyraxCommitment {
    let vars = mle_poly.num_vars();
    let pedersen_vars = (vars + 1) / 2;
    let pedersen_len = 1usize << pedersen_vars;
    assert_eq!(pedersen_len, params.0.len());

    let commitments: Vec<_> = mle_poly
        .hypercube_basis_ref()
        .chunks(pedersen_len)
        .map(|sub_hypercube| pedersen_vector_commit(params, sub_hypercube))
        .collect();

    HyraxCommitment(commitments)
}

#[allow(unused)]
pub(crate) fn hyrax_open<T: Transcript<BN254Fr>>(
    params: &PedersenParams,
    mle_poly: impl MultilinearExtension<BN254Fr>,
    eval_point: &[BN254Fr],
    transcript: &mut T,
) -> PedersenIPAProof {
    let vars = mle_poly.num_vars();
    let pedersen_vars = (vars + 1) / 2;
    let pedersen_len = 1usize << pedersen_vars;
    assert_eq!(pedersen_len, params.0.len());

    let mut local_mle = MultiLinearPoly::new(mle_poly.hypercube_basis());
    eval_point[pedersen_vars..]
        .iter()
        .rev()
        .for_each(|e| local_mle.fix_top_variable(*e));

    pedersen_ipa_prove(params, &local_mle.coeffs, transcript)
}

#[allow(unused)]
pub(crate) fn hyrax_verify<T: Transcript<BN254Fr>>(
    params: &PedersenParams,
    comm: &HyraxCommitment,
    eval_point: &[BN254Fr],
    eval: BN254Fr,
    proof: &PedersenIPAProof,
    transcript: &mut T,
) -> bool {
    let vars = eval_point.len();
    let pedersen_vars = (vars + 1) / 2;
    let pedersen_len = 1usize << pedersen_vars;
    assert_eq!(pedersen_len, params.0.len());

    let eq_combination = EqPolynomial::build_eq_x_r(&eval_point[pedersen_vars..]);
    let row_comm_g1: G1 = izip!(&comm.0, &eq_combination).map(|(c, e)| c * e).sum();
    let row_comm: G1Affine = row_comm_g1.into();

    let row_eqs = EqPolynomial::build_eq_x_r(&eval_point[..pedersen_len]);
    pedersen_ipa_verify(params, row_comm, proof, &row_eqs, eval, transcript)
}
