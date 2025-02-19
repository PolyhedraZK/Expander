use arith::{ExtensionField, FieldSerde};
use halo2curves::{ff::PrimeField, msm, CurveAffine};
use polynomials::{EqPolynomial, MultiLinearPoly, MultilinearExtension, RefMultiLinearPoly};

use crate::hyrax::{
    pedersen::{pedersen_commit, pedersen_setup},
    PedersenParams,
};

pub(crate) fn hyrax_setup<C: CurveAffine + FieldSerde>(
    local_vars: usize,
    rng: impl rand::RngCore,
) -> PedersenParams<C>
where
    C::Scalar: PrimeField,
{
    let pedersen_vars = (local_vars + 1) / 2;
    let pedersen_length = 1 << pedersen_vars;

    pedersen_setup(pedersen_length, rng)
}

#[derive(Clone, Debug, Default)]
pub struct HyraxCommitment<C: CurveAffine + FieldSerde>(pub Vec<C>);

impl<C: CurveAffine + FieldSerde> FieldSerde for HyraxCommitment<C> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, writer: W) -> arith::FieldSerdeResult<()> {
        self.0.serialize_into(writer)
    }

    fn deserialize_from<R: std::io::Read>(reader: R) -> arith::FieldSerdeResult<Self> {
        let buffer: Vec<C> = Vec::deserialize_from(reader)?;
        Ok(Self(buffer))
    }
}

pub(crate) fn hyrax_commit<C: CurveAffine + FieldSerde>(
    params: &PedersenParams<C>,
    mle_poly: &impl MultilinearExtension<C::Scalar>,
) -> HyraxCommitment<C>
where
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
{
    let vars = mle_poly.num_vars();
    let pedersen_vars = (vars + 1) / 2;
    let pedersen_len = 1usize << pedersen_vars;
    assert_eq!(pedersen_len, params.bases.len());

    let commitments: Vec<C> = mle_poly
        .hypercube_basis_ref()
        .chunks(pedersen_len)
        .map(|sub_hypercube| pedersen_commit(params, sub_hypercube))
        .collect();

    HyraxCommitment(commitments)
}

pub(crate) fn hyrax_open<C>(
    params: &PedersenParams<C>,
    mle_poly: &impl MultilinearExtension<C::Scalar>,
    eval_point: &[C::Scalar],
) -> (C::Scalar, Vec<C::Scalar>)
where
    C: CurveAffine + FieldSerde,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
{
    let vars = mle_poly.num_vars();
    let pedersen_vars = (vars + 1) / 2;
    let pedersen_len = 1usize << pedersen_vars;
    assert_eq!(pedersen_len, params.bases.len());

    let mut local_mle = MultiLinearPoly::new(mle_poly.hypercube_basis());
    eval_point[pedersen_vars..]
        .iter()
        .rev()
        .for_each(|e| local_mle.fix_top_variable(*e));

    let mut buffer = vec![C::Scalar::default(); local_mle.coeffs.len()];
    let final_eval = local_mle.evaluate_with_buffer(&eval_point[..pedersen_vars], &mut buffer);

    (final_eval, local_mle.coeffs)
}

pub(crate) fn hyrax_verify<C>(
    params: &PedersenParams<C>,
    comm: &HyraxCommitment<C>,
    eval_point: &[C::Scalar],
    eval: C::Scalar,
    proof: &Vec<C::Scalar>,
) -> bool
where
    C: CurveAffine + FieldSerde,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
{
    let vars = eval_point.len();
    let pedersen_vars = (vars + 1) / 2;
    let pedersen_len = 1usize << pedersen_vars;
    assert_eq!(pedersen_len, params.bases.len());

    let eq_combination: Vec<C::Scalar> = EqPolynomial::build_eq_x_r(&eval_point[pedersen_vars..]);
    let mut row_comm = C::Curve::default();
    msm::multiexp_serial(&eq_combination, &comm.0, &mut row_comm);

    if pedersen_commit(params, proof) != row_comm.into() {
        return false;
    }

    let mut scratch = vec![C::Scalar::default(); proof.len()];
    RefMultiLinearPoly::from_ref(proof)
        .evaluate_with_buffer(&eval_point[..pedersen_vars], &mut scratch)
        == eval
}
