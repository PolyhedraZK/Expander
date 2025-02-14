use arith::{ExtensionField, FieldSerde};
use halo2curves::{ff::PrimeField, CurveAffine};
use itertools::izip;
use polynomials::{EqPolynomial, MultiLinearPoly, MultilinearExtension};
use transcript::Transcript;

use crate::hyrax::{
    inner_prod_argument::{pedersen_ipa_prove, pedersen_ipa_verify},
    pedersen::{pedersen_setup, pedersen_vector_commit},
    PedersenIPAProof, PedersenParams,
};

pub(crate) fn hyrax_setup<C: CurveAffine + FieldSerde>(
    local_vars: usize,
    rng: impl rand::RngCore,
) -> PedersenParams<C>
where
    C::Scalar: PrimeField<Repr = [u8; 32]>,
{
    let pedersen_length = (local_vars + 1) / 2;

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
    C::Scalar: ExtensionField,
    C::ScalarExt: ExtensionField,
{
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

pub(crate) fn hyrax_open<C, T>(
    params: &PedersenParams<C>,
    mle_poly: &impl MultilinearExtension<C::Scalar>,
    eval_point: &[C::Scalar],
    transcript: &mut T,
) -> (C::Scalar, PedersenIPAProof<C>)
where
    C: CurveAffine + FieldSerde,
    T: Transcript<C::Scalar>,
    C::Scalar: ExtensionField,
    C::ScalarExt: ExtensionField,
{
    let vars = mle_poly.num_vars();
    let pedersen_vars = (vars + 1) / 2;
    let pedersen_len = 1usize << pedersen_vars;
    assert_eq!(pedersen_len, params.0.len());

    let mut local_mle = MultiLinearPoly::new(mle_poly.hypercube_basis());
    eval_point[pedersen_vars..]
        .iter()
        .rev()
        .for_each(|e| local_mle.fix_top_variable(*e));

    let mut buffer = vec![C::Scalar::default(); local_mle.coeffs.len()];
    let final_eval = local_mle.evaluate_with_buffer(&eval_point[..pedersen_len], &mut buffer);

    (
        final_eval,
        pedersen_ipa_prove(params, &local_mle.coeffs, transcript),
    )
}

pub(crate) fn hyrax_verify<C, T>(
    params: &PedersenParams<C>,
    comm: &HyraxCommitment<C>,
    eval_point: &[C::Scalar],
    eval: C::Scalar,
    proof: &PedersenIPAProof<C>,
    transcript: &mut T,
) -> bool
where
    C: CurveAffine + FieldSerde,
    T: Transcript<C::Scalar>,
    C::Scalar: ExtensionField,
    C::ScalarExt: ExtensionField,
{
    let vars = eval_point.len();
    let pedersen_vars = (vars + 1) / 2;
    let pedersen_len = 1usize << pedersen_vars;
    assert_eq!(pedersen_len, params.0.len());

    let eq_combination: Vec<C::Scalar> = EqPolynomial::build_eq_x_r(&eval_point[pedersen_vars..]);
    let row_comm_g1: C::Curve = izip!(&comm.0, &eq_combination).map(|(c, e)| *c * *e).sum();
    let row_comm: C = row_comm_g1.into();

    let row_eqs = EqPolynomial::build_eq_x_r(&eval_point[..pedersen_len]);
    pedersen_ipa_verify(params, row_comm, proof, &row_eqs, eval, transcript)
}
