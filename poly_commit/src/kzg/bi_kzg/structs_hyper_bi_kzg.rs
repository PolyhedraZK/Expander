use ark_ec::pairing::Pairing;
use derivative::Derivative;
use serdes::ExpSerde;

use crate::*;

#[derive(Debug, Clone, Derivative, ExpSerde)]
#[derivative(Default(bound = ""))]
pub struct HyperBiKZGOpening<E: Pairing>
where
    E::ScalarField: ExpSerde,
    E::G1Affine: Default + ExpSerde,
{
    pub folded_oracle_commitments: Vec<E::G1Affine>,

    pub aggregated_evals: HyperKZGAggregatedEvals<E>,
    pub leader_evals: HyperKZGExportedLocalEvals<E>,

    pub beta_x_commitment: E::G1Affine,
    pub beta_y_commitment: E::G1Affine,

    pub quotient_delta_x_commitment: E::G1Affine,
    pub quotient_delta_y_commitment: E::G1Affine,
}

impl<E: Pairing> From<HyperBiKZGOpening<E>> for HyperUniKZGOpening<E>
where
    E::ScalarField: ExpSerde,
    E::G1Affine: Default + ExpSerde,
{
    fn from(value: HyperBiKZGOpening<E>) -> Self {
        Self {
            folded_oracle_commitments: value.folded_oracle_commitments,
            evals_at_x: value.leader_evals,
            beta_x_commitment: value.beta_x_commitment,
            quotient_delta_x_commitment: value.quotient_delta_x_commitment,
        }
    }
}

impl<E: Pairing> From<HyperUniKZGOpening<E>> for HyperBiKZGOpening<E>
where
    E::ScalarField: ExpSerde,
    E::G1Affine: Default + ExpSerde,
{
    fn from(value: HyperUniKZGOpening<E>) -> Self {
        Self {
            folded_oracle_commitments: value.folded_oracle_commitments,
            leader_evals: value.evals_at_x,
            beta_x_commitment: value.beta_x_commitment,
            quotient_delta_x_commitment: value.quotient_delta_x_commitment,

            ..Default::default()
        }
    }
}
