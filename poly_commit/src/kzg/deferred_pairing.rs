use gkr_engine::DeferredCheck;
use halo2curves::group::prime::PrimeCurveAffine;
use halo2curves::group::Curve;
use halo2curves::{
    group::Group,
    pairing::{Engine, MillerLoopResult, MultiMillerLoop},
};

/// Deferred pairing checks
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairingAccumulator<E: Engine> {
    pub g1s: Vec<E::G1>,
    pub g2s: Vec<E::G2>,
}

impl<E> Default for PairingAccumulator<E>
where
    E: Engine,
{
    fn default() -> Self {
        Self {
            g1s: Vec::new(),
            g2s: Vec::new(),
        }
    }
}

impl<E: MultiMillerLoop> DeferredCheck for PairingAccumulator<E> {
    type AccumulatedValues = (E::G1, E::G2);

    fn accumulate(&mut self, accumulated_values: &(E::G1, E::G2)) {
        self.g1s.push(accumulated_values.0);
        self.g2s.push(accumulated_values.1);
    }

    fn final_check(&self) -> bool {
        if self.g1s.is_empty() || self.g2s.is_empty() {
            return true;
        }

        let mut g1_affines = vec![<E as Engine>::G1Affine::identity(); self.g1s.len()];
        E::G1::batch_normalize(&self.g1s, &mut g1_affines);

        let mut g2_affines = vec![<E as Engine>::G2Affine::identity(); self.g2s.len()];
        E::G2::batch_normalize(&self.g2s, &mut g2_affines);
        let g2_prepared: Vec<E::G2Prepared> =
            g2_affines.iter().map(|&g2| g2.into()).collect::<Vec<_>>();

        let g1g2_pairs = g1_affines
            .iter()
            .zip(g2_prepared.iter())
            .collect::<Vec<_>>();

        let gt_result = E::multi_miller_loop(g1g2_pairs.as_slice());
        gt_result.final_exponentiation().is_identity().into()
    }
}
