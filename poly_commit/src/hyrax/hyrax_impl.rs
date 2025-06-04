use arith::ExtensionField;
use halo2curves::{ff::PrimeField, group::UncompressedEncoding, msm, CurveAffine};
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
    C::Base: PrimeField<Repr = [u8; 32]>,
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
pub struct HyraxCommitment<C>(pub Vec<C>)
where
    C: CurveAffine + ExpSerde + UncompressedEncoding;

#[derive(Clone, Debug, Default)]
pub struct HyraxOpening<C>(pub Vec<C::Scalar>)
where
    C: CurveAffine + ExpSerde + UncompressedEncoding;

impl<C> ExpSerde for HyraxCommitment<C>
where
    C: CurveAffine + ExpSerde + UncompressedEncoding,
{
    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> serdes::SerdeResult<()> {
        self.0.len().serialize_into(&mut writer)?;
        for c in self.0.iter() {
            let uncompressed = UncompressedEncoding::to_uncompressed(c);
            writer.write_all(uncompressed.as_ref())?;
        }
        Ok(())
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> serdes::SerdeResult<Self> {
        let num_elements = usize::deserialize_from(&mut reader)?;
        let mut uncompressed = <C as UncompressedEncoding>::Uncompressed::default();

        let mut elements = Vec::with_capacity(num_elements);
        for _ in 0..num_elements {
            reader.read_exact(uncompressed.as_mut())?;
            elements.push(
                C::from_uncompressed_unchecked(&uncompressed)
                    .into_option()
                    .ok_or(serdes::SerdeError::DeserializeError)?,
            );
        }
        Ok(Self(elements))
    }
}

impl<C> ExpSerde for HyraxOpening<C>
where
    C: CurveAffine + ExpSerde + UncompressedEncoding,
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

pub(crate) fn hyrax_commit<C>(
    params: &PedersenParams<C>,
    mle_poly: &impl MultilinearExtension<C::Scalar>,
) -> HyraxCommitment<C>
where
    C: CurveAffine + ExpSerde + UncompressedEncoding,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
    C::Base: PrimeField<Repr = [u8; 32]>,
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
    C: CurveAffine + ExpSerde + UncompressedEncoding,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
    C::Base: PrimeField<Repr = [u8; 32]>,
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
    C: CurveAffine + ExpSerde + UncompressedEncoding,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
    C::Base: PrimeField<Repr = [u8; 32]>,
{
    let pedersen_len = params.msm_len();
    let pedersen_vars = pedersen_len.ilog2() as usize;

    let eq_combination: Vec<C::Scalar> = EqPolynomial::build_eq_x_r(&eval_point[pedersen_vars..]);
    let row_comm = msm::best_multiexp(&eq_combination, &comm.0);

    if pedersen_commit(params, &proof.0) != row_comm.into() {
        return false;
    }

    let mut scratch = vec![C::Scalar::default(); proof.0.len()];
    eval == RefMultiLinearPoly::from_ref(&proof.0)
        .evaluate_with_buffer(&eval_point[..pedersen_vars], &mut scratch)
}
