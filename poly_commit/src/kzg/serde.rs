use halo2curves::{pairing::Engine, CurveAffine};
use serdes::{ExpSerde, SerdeResult};

use crate::*;

impl<E: Engine> ExpSerde for KZGCommitment<E>
where
    E::G1Affine: ExpSerde + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    const SERIALIZED_SIZE: usize = <E::G1Affine as ExpSerde>::SERIALIZED_SIZE;

    fn serialize_into<W: std::io::Write>(&self, writer: W) -> SerdeResult<()> {
        self.0.serialize_into(writer)
    }

    fn deserialize_from<R: std::io::Read>(reader: R) -> SerdeResult<Self> {
        Ok(Self(<E::G1Affine as ExpSerde>::deserialize_from(reader)?))
    }
}

impl<E: Engine> ExpSerde for CoefFormUniKZGSRS<E>
where
    E::G1Affine: ExpSerde + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::G2Affine: ExpSerde + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.powers_of_tau.serialize_into(&mut writer)?;
        self.tau_g2.serialize_into(&mut writer)
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> SerdeResult<Self> {
        let powers_of_tau: Vec<E::G1Affine> = Vec::deserialize_from(&mut reader)?;
        let tau_g2: E::G2Affine = E::G2Affine::deserialize_from(&mut reader)?;
        Ok(Self {
            powers_of_tau,
            tau_g2,
        })
    }
}

impl<E: Engine> ExpSerde for UniKZGVerifierParams<E>
where
    E::G2Affine: ExpSerde,
{
    const SERIALIZED_SIZE: usize = <E::G2Affine as ExpSerde>::SERIALIZED_SIZE;

    fn serialize_into<W: std::io::Write>(&self, writer: W) -> SerdeResult<()> {
        self.tau_g2.serialize_into(writer)
    }

    fn deserialize_from<R: std::io::Read>(reader: R) -> SerdeResult<Self> {
        Ok(Self {
            tau_g2: <E::G2Affine as ExpSerde>::deserialize_from(reader)?,
        })
    }
}

impl<E: Engine> ExpSerde for CoefFormBiKZGLocalSRS<E>
where
    E::G1Affine: ExpSerde + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::G2Affine: ExpSerde + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.tau_x_srs.serialize_into(&mut writer)?;
        self.tau_y_srs.serialize_into(&mut writer)
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> SerdeResult<Self> {
        let tau_x_srs = CoefFormUniKZGSRS::deserialize_from(&mut reader)?;
        let tau_y_srs = CoefFormUniKZGSRS::deserialize_from(&mut reader)?;

        Ok(Self {
            tau_x_srs,
            tau_y_srs,
        })
    }
}

impl<E: Engine> ExpSerde for BiKZGVerifierParam<E>
where
    E::G2Affine: ExpSerde + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    const SERIALIZED_SIZE: usize = 2 * <E::G2Affine as ExpSerde>::SERIALIZED_SIZE;

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.tau_x_g2.serialize_into(&mut writer)?;
        self.tau_y_g2.serialize_into(&mut writer)
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> SerdeResult<Self> {
        let tau_x_g2 = E::G2Affine::deserialize_from(&mut reader)?;
        let tau_y_g2 = E::G2Affine::deserialize_from(&mut reader)?;

        Ok(Self { tau_x_g2, tau_y_g2 })
    }
}

impl<E: Engine> ExpSerde for HyperKZGExportedLocalEvals<E>
where
    E::Fr: ExpSerde,
{
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.beta_x2_eval.serialize_into(&mut writer)?;
        self.pos_beta_x_evals.serialize_into(&mut writer)?;
        self.neg_beta_x_evals.serialize_into(&mut writer)
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> SerdeResult<Self> {
        let beta_x2_eval: E::Fr = E::Fr::deserialize_from(&mut reader)?;
        let pos_beta_x_evals: Vec<E::Fr> = Vec::deserialize_from(&mut reader)?;
        let neg_beta_x_evals: Vec<E::Fr> = Vec::deserialize_from(&mut reader)?;

        Ok(Self {
            beta_x2_eval,
            pos_beta_x_evals,
            neg_beta_x_evals,
        })
    }
}

impl<E: Engine> ExpSerde for HyperKZGOpening<E>
where
    E::Fr: ExpSerde,
    E::G1Affine: ExpSerde + Default,
{
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.folded_oracle_commitments.serialize_into(&mut writer)?;
        self.evals_at_x.serialize_into(&mut writer)?;
        self.beta_x_commitment.serialize_into(&mut writer)?;
        self.quotient_delta_x_commitment.serialize_into(&mut writer)
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> SerdeResult<Self> {
        let folded_oracle_commitments: Vec<E::G1Affine> = Vec::deserialize_from(&mut reader)?;
        let evals_at_x = HyperKZGExportedLocalEvals::<E>::deserialize_from(&mut reader)?;
        let beta_x_commitment = E::G1Affine::deserialize_from(&mut reader)?;
        let quotient_delta_x_commitment = E::G1Affine::deserialize_from(&mut reader)?;

        Ok(Self {
            folded_oracle_commitments,
            evals_at_x,
            beta_x_commitment,
            quotient_delta_x_commitment,
        })
    }
}

impl<E: Engine> ExpSerde for HyperKZGAggregatedEvals<E>
where
    E::Fr: ExpSerde,
{
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.beta_y2_evals.serialize_into(&mut writer)?;
        self.pos_beta_y_evals.serialize_into(&mut writer)?;
        self.neg_beta_y_evals.serialize_into(&mut writer)
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> SerdeResult<Self> {
        let beta_y2_evals = HyperKZGExportedLocalEvals::<E>::deserialize_from(&mut reader)?;
        let pos_beta_y_evals = HyperKZGExportedLocalEvals::<E>::deserialize_from(&mut reader)?;
        let neg_beta_y_evals = HyperKZGExportedLocalEvals::<E>::deserialize_from(&mut reader)?;

        Ok(Self {
            beta_y2_evals,
            pos_beta_y_evals,
            neg_beta_y_evals,
        })
    }
}

impl<E: Engine> ExpSerde for HyperBiKZGOpening<E>
where
    E::Fr: ExpSerde,
    E::G1Affine: ExpSerde + Default,
{
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.folded_oracle_commitments.serialize_into(&mut writer)?;

        self.aggregated_evals.serialize_into(&mut writer)?;
        self.leader_evals.serialize_into(&mut writer)?;

        self.beta_x_commitment.serialize_into(&mut writer)?;
        self.beta_y_commitment.serialize_into(&mut writer)?;

        self.quotient_delta_x_commitment
            .serialize_into(&mut writer)?;
        self.quotient_delta_y_commitment.serialize_into(&mut writer)
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> SerdeResult<Self> {
        let folded_oracle_commitments: Vec<E::G1Affine> = Vec::deserialize_from(&mut reader)?;

        let aggregated_evals = HyperKZGAggregatedEvals::<E>::deserialize_from(&mut reader)?;
        let leader_evals = HyperKZGExportedLocalEvals::<E>::deserialize_from(&mut reader)?;

        let beta_x_commitment = E::G1Affine::deserialize_from(&mut reader)?;
        let beta_y_commitment = E::G1Affine::deserialize_from(&mut reader)?;

        let quotient_delta_x_commitment = E::G1Affine::deserialize_from(&mut reader)?;
        let quotient_delta_y_commitment = E::G1Affine::deserialize_from(&mut reader)?;

        Ok(Self {
            folded_oracle_commitments,

            aggregated_evals,
            leader_evals,

            beta_x_commitment,
            beta_y_commitment,

            quotient_delta_x_commitment,
            quotient_delta_y_commitment,
        })
    }
}
