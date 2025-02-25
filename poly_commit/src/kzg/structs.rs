use std::ops::{Index, IndexMut};

use arith::{ExtensionField, FieldSerde};
use halo2curves::{pairing::Engine, CurveAffine};
use itertools::izip;
use transcript::Transcript;

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KZGCommitment<E: Engine>(pub E::G1Affine);

impl<E: Engine> Default for KZGCommitment<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    fn default() -> Self {
        Self(E::G1Affine::default())
    }
}

impl<E: Engine> FieldSerde for KZGCommitment<E>
where
    E::G1Affine: FieldSerde,
{
    const SERIALIZED_SIZE: usize = <E::G1Affine as FieldSerde>::SERIALIZED_SIZE;

    fn serialize_into<W: std::io::Write>(&self, writer: W) -> arith::FieldSerdeResult<()> {
        self.0.serialize_into(writer)
    }

    fn deserialize_from<R: std::io::Read>(reader: R) -> arith::FieldSerdeResult<Self> {
        Ok(Self(<E::G1Affine as FieldSerde>::deserialize_from(reader)?))
    }
}

/// Structured reference string for univariate KZG polynomial commitment scheme.
/// The univariate polynomial here is of coefficient form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoefFormUniKZGSRS<E: Engine> {
    /// power of \tau times the generators of G1, yielding
    /// \tau^i over G1 with i ranging in \[ 0, 2^n - 1 \]
    pub powers_of_tau: Vec<E::G1Affine>,
    /// \tau over G2
    pub tau_g2: E::G2Affine,
}

impl<E: Engine> Default for CoefFormUniKZGSRS<E>
where
    E::G2Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    fn default() -> Self {
        Self {
            powers_of_tau: Vec::default(),
            tau_g2: E::G2Affine::default(),
        }
    }
}

impl<E: Engine> FieldSerde for CoefFormUniKZGSRS<E>
where
    E::G1Affine: FieldSerde,
    E::G2Affine: FieldSerde,
{
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> arith::FieldSerdeResult<()> {
        self.powers_of_tau.serialize_into(&mut writer)?;
        self.tau_g2.serialize_into(&mut writer)
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> arith::FieldSerdeResult<Self> {
        let powers_of_tau: Vec<E::G1Affine> = Vec::deserialize_from(&mut reader)?;
        let tau_g2: E::G2Affine = E::G2Affine::deserialize_from(&mut reader)?;
        Ok(Self {
            powers_of_tau,
            tau_g2,
        })
    }
}

impl<E: Engine> StructuredReferenceString for CoefFormUniKZGSRS<E>
where
    <E as Engine>::G1Affine: FieldSerde,
    <E as Engine>::G2Affine: FieldSerde,
{
    type PKey = CoefFormUniKZGSRS<E>;
    type VKey = UniKZGVerifierParams<E>;

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        (self.clone(), From::from(&self))
    }
}

/// Univariate KZG PCS verifier's params.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UniKZGVerifierParams<E: Engine> {
    /// \tau over G2
    pub tau_g2: E::G2Affine,
}

impl<E: Engine> FieldSerde for UniKZGVerifierParams<E>
where
    E::G2Affine: FieldSerde,
{
    const SERIALIZED_SIZE: usize = <E::G2Affine as FieldSerde>::SERIALIZED_SIZE;

    fn serialize_into<W: std::io::Write>(&self, writer: W) -> arith::FieldSerdeResult<()> {
        self.tau_g2.serialize_into(writer)
    }

    fn deserialize_from<R: std::io::Read>(reader: R) -> arith::FieldSerdeResult<Self> {
        Ok(Self {
            tau_g2: <E::G2Affine as FieldSerde>::deserialize_from(reader)?,
        })
    }
}

impl<E: Engine> From<&CoefFormUniKZGSRS<E>> for UniKZGVerifierParams<E> {
    fn from(value: &CoefFormUniKZGSRS<E>) -> Self {
        Self {
            tau_g2: value.tau_g2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HyperKZGOpening<E: Engine> {
    pub folded_oracle_commitments: Vec<E::G1Affine>,
    pub f_beta2: E::Fr,
    pub evals_at_beta: Vec<E::Fr>,
    pub evals_at_neg_beta: Vec<E::Fr>,
    pub beta_commitment: E::G1Affine,
    pub tau_vanishing_commitment: E::G1Affine,
}

impl<E: Engine> Default for HyperKZGOpening<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    fn default() -> Self {
        Self {
            folded_oracle_commitments: Vec::default(),
            f_beta2: E::Fr::default(),
            evals_at_beta: Vec::default(),
            evals_at_neg_beta: Vec::default(),
            beta_commitment: E::G1Affine::default(),
            tau_vanishing_commitment: E::G1Affine::default(),
        }
    }
}

impl<E: Engine> FieldSerde for HyperKZGOpening<E>
where
    E::Fr: FieldSerde,
    E::G1Affine: FieldSerde,
    E::G2Affine: FieldSerde,
{
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> arith::FieldSerdeResult<()> {
        self.folded_oracle_commitments.serialize_into(&mut writer)?;
        self.f_beta2.serialize_into(&mut writer)?;
        self.evals_at_beta.serialize_into(&mut writer)?;
        self.evals_at_neg_beta.serialize_into(&mut writer)?;
        self.beta_commitment.serialize_into(&mut writer)?;
        self.tau_vanishing_commitment.serialize_into(&mut writer)
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> arith::FieldSerdeResult<Self> {
        let folded_oracle_commitments: Vec<E::G1Affine> = Vec::deserialize_from(&mut reader)?;
        let f_beta2: E::Fr = E::Fr::deserialize_from(&mut reader)?;
        let evals_at_beta: Vec<E::Fr> = Vec::deserialize_from(&mut reader)?;
        let evals_at_neg_beta: Vec<E::Fr> = Vec::deserialize_from(&mut reader)?;
        let beta_commitment: E::G1Affine = E::G1Affine::deserialize_from(&mut reader)?;
        let tau_vanishing_commitment: E::G1Affine = E::G1Affine::deserialize_from(&mut reader)?;

        Ok(Self {
            folded_oracle_commitments,
            f_beta2,
            evals_at_beta,
            evals_at_neg_beta,
            beta_commitment,
            tau_vanishing_commitment,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoefFormBiKZGLocalSRS<E: Engine> {
    pub tau_x_srs: CoefFormUniKZGSRS<E>,
    pub tau_y_srs: CoefFormUniKZGSRS<E>,
}

/// Bivariate KZG PCS verifier's params.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct BiKZGVerifierParam<E: Engine> {
    /// tau_0 over G2.
    pub tau_x_g2: E::G2Affine,
    /// tau_y over G2.
    pub tau_y_g2: E::G2Affine,
}

impl<E: Engine> From<&CoefFormBiKZGLocalSRS<E>> for BiKZGVerifierParam<E> {
    fn from(srs: &CoefFormBiKZGLocalSRS<E>) -> Self {
        Self {
            tau_x_g2: srs.tau_x_srs.tau_g2,
            tau_y_g2: srs.tau_y_srs.tau_g2,
        }
    }
}

/// Proof for Bi-KZG polynomial commitment scheme.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct BiKZGProof<E: Engine> {
    pub quotient_x: E::G1Affine,
    pub quotient_y: E::G1Affine,
}

#[derive(Clone, Debug)]
pub(crate) struct HyperKZGLocalEvals<E: Engine> {
    pub(crate) beta2_evals: Vec<E::Fr>,
    pub(crate) pos_beta_evals: Vec<E::Fr>,
    pub(crate) neg_beta_evals: Vec<E::Fr>,
}

impl<E: Engine> Default for HyperKZGLocalEvals<E> {
    fn default() -> Self {
        Self {
            beta2_evals: Vec::default(),
            pos_beta_evals: Vec::default(),
            neg_beta_evals: Vec::default(),
        }
    }
}

impl<E: Engine> HyperKZGLocalEvals<E> {
    pub(crate) fn new_from_beta2_evals(beta2_eval: E::Fr) -> Self {
        Self {
            beta2_evals: vec![beta2_eval],
            pos_beta_evals: Vec::new(),
            neg_beta_evals: Vec::new(),
        }
    }

    // NOTE(HS) introduce a gamma <- RO, s.t., we aggregate the evaluations
    // with the power series of gamma through linear combination.
    //
    // THE ASSUMPTION here is: pos/neg beta evals are from polynomial oracles
    // that are folded until degree being 1, i.e., 2 coefficients, while the
    // beta2 evals folds one step more, that the polynomial eventually has
    // degree 0, and thus we assume that beta2_evals is one element more than
    // pos/neg beta evals.
    //
    // The return order is the evals at beta2, beta, and -beta.
    pub(crate) fn gamma_aggregate_evals(&self, gamma: E::Fr) -> (E::Fr, E::Fr, E::Fr) {
        assert_eq!(self.pos_beta_evals.len(), self.neg_beta_evals.len());
        assert_eq!(self.pos_beta_evals.len() + 1, self.beta2_evals.len());

        let gamma_pow_series = powers_series(&gamma, self.pos_beta_evals.len());
        let v_beta2 = univariate_evaluate(&self.beta2_evals, &gamma_pow_series);
        let v_beta = univariate_evaluate(&self.pos_beta_evals, &gamma_pow_series);
        let v_neg_beta = univariate_evaluate(&self.neg_beta_evals, &gamma_pow_series);

        (v_beta2, v_beta, v_neg_beta)
    }

    pub(crate) fn interpolate_degree2_aggregated_evals(
        &self,
        beta: E::Fr,
        gamma: E::Fr,
    ) -> [E::Fr; 3] {
        let beta2 = beta * beta;
        let (v_beta2, v_beta, v_neg_beta) = self.gamma_aggregate_evals(gamma);
        coeff_form_degree2_lagrange([beta, -beta, beta2], [v_beta, v_neg_beta, v_beta2])
    }

    // NOTE(HS) the same assumption applies here, that last beta2 eval is the
    // multilinear polynomial eval, as it folds to univariate poly of degree 0.
    pub(crate) fn multilinear_final_eval(&self) -> E::Fr {
        self.beta2_evals[self.beta2_evals.len() - 1]
    }

    pub(crate) fn append_to_transcript<T>(&self, fs_transcript: &mut T)
    where
        T: Transcript<E::Fr>,
        E::Fr: ExtensionField,
    {
        fs_transcript.append_field_element(&self.beta2_evals[0]);
        izip!(&self.pos_beta_evals, &self.neg_beta_evals).for_each(|(beta_eval, neg_beta_eval)| {
            fs_transcript.append_field_element(beta_eval);
            fs_transcript.append_field_element(neg_beta_eval);
        });
    }
}

#[allow(unused)]
#[derive(Clone, Debug)]
pub(crate) struct HyperKZGExportedLocalEvals<E: Engine> {
    pub(crate) beta_x2_eval: E::Fr,
    pub(crate) pos_beta_x_evals: Vec<E::Fr>,
    pub(crate) neg_beta_x_evals: Vec<E::Fr>,
}

#[allow(unused)]
impl<E: Engine> HyperKZGExportedLocalEvals<E> {
    pub(crate) fn new(evals_num: usize) -> Self {
        Self {
            beta_x2_eval: E::Fr::default(),
            pos_beta_x_evals: vec![E::Fr::default(); evals_num],
            neg_beta_x_evals: vec![E::Fr::default(); evals_num],
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.pos_beta_x_evals.len() + self.neg_beta_x_evals.len() + 1
    }

    pub(crate) fn append_to_transcript<T>(&self, fs_transcript: &mut T)
    where
        T: Transcript<E::Fr>,
        E::Fr: ExtensionField,
    {
        fs_transcript.append_field_element(&self.beta_x2_eval);
        izip!(&self.pos_beta_x_evals, &self.neg_beta_x_evals).for_each(
            |(beta_eval, neg_beta_eval)| {
                fs_transcript.append_field_element(beta_eval);
                fs_transcript.append_field_element(neg_beta_eval);
            },
        );
    }
}

impl<E: Engine> Index<usize> for HyperKZGExportedLocalEvals<E> {
    type Output = E::Fr;

    fn index(&self, index: usize) -> &Self::Output {
        assert_eq!(self.pos_beta_x_evals.len(), self.neg_beta_x_evals.len());
        assert!(!self.pos_beta_x_evals.is_empty());

        let evals_len = self.pos_beta_x_evals.len();

        if index < evals_len {
            &self.pos_beta_x_evals[index]
        } else if index < 2 * evals_len {
            &self.neg_beta_x_evals[index - evals_len]
        } else if index == 2 * evals_len {
            &self.beta_x2_eval
        } else {
            unreachable!()
        }
    }
}

impl<E: Engine> IndexMut<usize> for HyperKZGExportedLocalEvals<E> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert_eq!(self.pos_beta_x_evals.len(), self.neg_beta_x_evals.len());
        assert!(!self.pos_beta_x_evals.is_empty());

        let evals_len = self.pos_beta_x_evals.len();

        if index < evals_len {
            &mut self.pos_beta_x_evals[index]
        } else if index < 2 * evals_len {
            &mut self.neg_beta_x_evals[index - evals_len]
        } else if index == 2 * evals_len {
            &mut self.beta_x2_eval
        } else {
            unreachable!()
        }
    }
}

impl<E: Engine> From<HyperKZGLocalEvals<E>> for HyperKZGExportedLocalEvals<E> {
    fn from(value: HyperKZGLocalEvals<E>) -> Self {
        Self {
            beta_x2_eval: value.beta2_evals[0],
            pos_beta_x_evals: value.pos_beta_evals,
            neg_beta_x_evals: value.neg_beta_evals,
        }
    }
}

#[allow(unused)]
#[derive(Clone, Debug)]
pub(crate) struct HyperKZGAggregatedEvals<E: Engine> {
    pub(crate) beta_y2_evals: HyperKZGExportedLocalEvals<E>,
    pub(crate) pos_beta_y_evals: HyperKZGExportedLocalEvals<E>,
    pub(crate) neg_beta_y_evals: HyperKZGExportedLocalEvals<E>,
}

#[allow(unused)]
impl<E: Engine> HyperKZGAggregatedEvals<E> {
    pub(crate) fn new_from_exported_evals(
        exported_evals: &[HyperKZGExportedLocalEvals<E>],
        beta_y: E::Fr,
    ) -> Self {
        let evals_len = exported_evals[0].pos_beta_x_evals.len();
        let num_local_evals = exported_evals[0].len();
        let num_parties = exported_evals.len();

        assert!(num_parties >= 2 && num_parties.is_power_of_two());

        let mut aggregated = Self {
            beta_y2_evals: HyperKZGExportedLocalEvals::new(evals_len),
            pos_beta_y_evals: HyperKZGExportedLocalEvals::new(evals_len),
            neg_beta_y_evals: HyperKZGExportedLocalEvals::new(evals_len),
        };

        let beta_y2 = beta_y * beta_y;
        let beta_y2_pow_series = powers_series(&beta_y2, num_parties);
        let pos_beta_y_pow_series = powers_series(&beta_y, num_parties);
        let neg_beta_y_pow_series = powers_series(&(-beta_y2), num_parties);

        (0..num_local_evals).for_each(|i| {
            let y_poly: Vec<E::Fr> = exported_evals.iter().map(|e| e[i]).collect();

            aggregated.beta_y2_evals[i] = univariate_evaluate(&y_poly, &beta_y2_pow_series);
            aggregated.pos_beta_y_evals[i] = univariate_evaluate(&y_poly, &pos_beta_y_pow_series);
            aggregated.neg_beta_y_evals[i] = univariate_evaluate(&y_poly, &neg_beta_y_pow_series);
        });

        aggregated
    }
}
