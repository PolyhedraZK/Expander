use arith::ExtensionField;
use derivative::Derivative;
use gkr_engine::Transcript;
use halo2curves::{ff::Field, pairing::Engine};
use itertools::izip;
use serdes::ExpSerde;

use crate::*;

#[derive(Clone, Debug, Derivative, ExpSerde)]
#[derivative(Default(bound = ""))]
pub struct HyperKZGExportedLocalEvals<E: Engine>
where
    E::Fr: ExpSerde,
{
    pub beta_x2_eval: E::Fr,
    pub pos_beta_x_evals: Vec<E::Fr>,
    pub neg_beta_x_evals: Vec<E::Fr>,
}

impl<E: Engine> HyperKZGExportedLocalEvals<E>
where
    E::Fr: ExpSerde,
{
    pub(crate) fn append_to_transcript<T>(&self, fs_transcript: &mut T)
    where
        T: Transcript,
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

#[derive(Clone, Debug, Derivative, ExpSerde)]
#[derivative(Default(bound = ""))]
pub struct HyperUniKZGOpening<E: Engine>
where
    E::Fr: ExpSerde,
    E::G1Affine: Default + ExpSerde,
{
    pub folded_oracle_commitments: Vec<E::G1Affine>,
    pub evals_at_x: HyperKZGExportedLocalEvals<E>,
    pub beta_x_commitment: E::G1Affine,
    pub quotient_delta_x_commitment: E::G1Affine,
}

#[derive(Clone, Debug, Derivative, ExpSerde)]
#[derivative(Default(bound = ""))]
pub(crate) struct HyperKZGLocalEvals<E: Engine>
where
    E::Fr: ExpSerde,
{
    pub(crate) beta2_evals: Vec<E::Fr>,
    pub(crate) pos_beta_evals: Vec<E::Fr>,
    pub(crate) neg_beta_evals: Vec<E::Fr>,
}

impl<E: Engine> HyperKZGLocalEvals<E>
where
    E::Fr: ExtensionField,
{
    pub(crate) fn new_from_beta2_evals(beta2_eval: E::Fr) -> Self {
        Self {
            beta2_evals: vec![beta2_eval],
            pos_beta_evals: Vec::new(),
            neg_beta_evals: Vec::new(),
        }
    }

    pub(crate) fn new_from_exported_evals(
        exported: &HyperKZGExportedLocalEvals<E>,
        alphas: &[E::Fr],
        beta: E::Fr,
    ) -> Self {
        let beta_inv = beta.invert().unwrap();
        let two_inv = E::Fr::ONE.double().invert().unwrap();

        let mut local_evals = Self::new_from_beta2_evals(exported.beta_x2_eval);

        izip!(
            &exported.pos_beta_x_evals,
            &exported.neg_beta_x_evals,
            alphas
        )
        .for_each(|(pos_beta_x_eval, neg_beta_x_eval, alpha)| {
            let beta2_eval = two_inv
                * ((*pos_beta_x_eval + *neg_beta_x_eval) * (E::Fr::ONE - alpha)
                    + (*pos_beta_x_eval - *neg_beta_x_eval) * beta_inv * alpha);

            local_evals.beta2_evals.push(beta2_eval);
            local_evals.pos_beta_evals.push(*pos_beta_x_eval);
            local_evals.neg_beta_evals.push(*neg_beta_x_eval);
        });

        local_evals
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
        T: Transcript,
    {
        fs_transcript.append_field_element(&self.beta2_evals[0]);
        izip!(&self.pos_beta_evals, &self.neg_beta_evals).for_each(|(beta_eval, neg_beta_eval)| {
            fs_transcript.append_field_element(beta_eval);
            fs_transcript.append_field_element(neg_beta_eval);
        });
    }
}

impl<E: Engine> From<HyperKZGLocalEvals<E>> for HyperKZGExportedLocalEvals<E>
where
    E::Fr: ExpSerde,
{
    fn from(value: HyperKZGLocalEvals<E>) -> Self {
        Self {
            beta_x2_eval: value.beta2_evals[0],
            pos_beta_x_evals: value.pos_beta_evals,
            neg_beta_x_evals: value.neg_beta_evals,
        }
    }
}
