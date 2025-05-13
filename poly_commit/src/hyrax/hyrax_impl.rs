use arith::ExtensionField;
use halo2curves::{ff::PrimeField, msm, CurveAffine};
use polynomials::{
    EqPolynomial, MultilinearExtension, MutRefMultiLinearPoly, MutableMultilinearExtension,
    RefMultiLinearPoly,
};
use serdes::ExpSerde;

use crate::hyrax::{
    pedersen::{pedersen_commit, pedersen_setup},
    PedersenParams,
};

pub(crate) fn hyrax_setup<C: CurveAffine + ExpSerde>(
    local_vars: usize,
    mpi_vars: usize,
    rng: impl rand::RngCore,
) -> PedersenParams<C>
where
    C::Scalar: PrimeField,
{
    let pedersen_vars = {
        let total_vars = mpi_vars + local_vars;
        let squared_row_var = total_vars.div_ceil(2);

        if mpi_vars + squared_row_var > total_vars {
            total_vars - mpi_vars
        } else {
            squared_row_var
        }
    };

    let pedersen_length = 1 << pedersen_vars;

    pedersen_setup(pedersen_length, rng)
}

#[derive(Clone, Debug, Default)]
pub struct HyraxCommitment<C: CurveAffine + ExpSerde>(pub Vec<C>);

#[derive(Clone, Debug, Default)]
pub struct HyraxOpening<C: CurveAffine + ExpSerde>(pub Vec<C::Scalar>);

impl<C: CurveAffine + ExpSerde> ExpSerde for HyraxCommitment<C> {
    fn serialize_into<W: std::io::Write>(&self, writer: W) -> serdes::SerdeResult<()> {
        self.0.serialize_into(writer)
    }

    fn deserialize_from<R: std::io::Read>(reader: R) -> serdes::SerdeResult<Self> {
        let buffer: Vec<C> = <Vec<C> as ExpSerde>::deserialize_from(reader)?;
        Ok(Self(buffer))
    }
}

impl<C: CurveAffine + ExpSerde> ExpSerde for HyraxOpening<C>
where
    C::Scalar: ExpSerde,
{
    fn serialize_into<W: std::io::Write>(&self, writer: W) -> serdes::SerdeResult<()> {
        self.0.serialize_into(writer)
    }

    fn deserialize_from<R: std::io::Read>(reader: R) -> serdes::SerdeResult<Self> {
        let buffer: Vec<C::Scalar> = <Vec<C::Scalar> as ExpSerde>::deserialize_from(reader)?;
        Ok(Self(buffer))
    }
}

pub(crate) fn hyrax_commit<C: CurveAffine + ExpSerde>(
    params: &PedersenParams<C>,
    mle_poly: &impl MultilinearExtension<C::Scalar>,
) -> HyraxCommitment<C>
where
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
{
    let commitments: Vec<C> = mle_poly
        .hypercube_basis_ref()
        .chunks(params.msm_len())
        .map(|sub_hypercube| pedersen_commit(params, sub_hypercube))
        .collect();

    HyraxCommitment(commitments)
}

// NOTE(HS) the hyrax opening returns an eval and an opening against the eval_point on input.
pub(crate) fn hyrax_open<C>(
    params: &PedersenParams<C>,
    mle_poly: &impl MultilinearExtension<C::Scalar>,
    eval_point: &[C::Scalar],
) -> (C::Scalar, HyraxOpening<C>)
where
    C: CurveAffine + ExpSerde,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
{
    let pedersen_len = params.msm_len();
    let pedersen_vars = pedersen_len.ilog2() as usize;

    let mut local_basis = mle_poly.hypercube_basis();
    let mut local_mle = MutRefMultiLinearPoly::from_ref(&mut local_basis);
    local_mle.fix_variables(&eval_point[pedersen_vars..]);

    let mut buffer = vec![C::Scalar::default(); local_mle.coeffs.len()];
    let final_eval = local_mle.evaluate_with_buffer(&eval_point[..pedersen_vars], &mut buffer);

    (final_eval, HyraxOpening(local_basis))
}

pub(crate) fn hyrax_verify<C>(
    params: &PedersenParams<C>,
    comm: &HyraxCommitment<C>,
    eval_point: &[C::Scalar],
    eval: C::Scalar,
    proof: &HyraxOpening<C>,
) -> bool
where
    C: CurveAffine + ExpSerde,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
{
    let pedersen_len = params.msm_len();
    let pedersen_vars = pedersen_len.ilog2() as usize;

    let eq_combination: Vec<C::Scalar> = EqPolynomial::build_eq_x_r(&eval_point[pedersen_vars..]);
    let mut row_comm = C::Curve::default();
    msm::multiexp_serial(&eq_combination, &comm.0, &mut row_comm);

    if pedersen_commit(params, &proof.0) != row_comm.into() {
        return false;
    }

    let mut scratch = vec![C::Scalar::default(); proof.0.len()];
    eval == RefMultiLinearPoly::from_ref(&proof.0)
        .evaluate_with_buffer(&eval_point[..pedersen_vars], &mut scratch)
}
