use halo2curves::group::prime::PrimeCurveAffine;
use halo2curves::group::Curve;
use halo2curves::{
    group::Group,
    pairing::{Engine, MillerLoopResult, MultiMillerLoop},
};

/// Deferred pairing checks
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PairingAccumulator<E: Engine> {
    pub g1s: Vec<E::G1>,
    pub g2s: Vec<E::G2>,
}

pub trait DeferredPairingCheck {
    /// Data type to be accumulated
    type AccumulatedValues;

    /// Add a new pairing check to the accumulator
    fn add_pairing_check(&mut self, _accumulated_values: &Self::AccumulatedValues) {}

    /// Check if all pairings are valid
    fn check_pairings(&self) -> bool {
        true
    }
}

impl DeferredPairingCheck for () {
    type AccumulatedValues = ();
}

impl<E: MultiMillerLoop> DeferredPairingCheck for PairingAccumulator<E> {
    type AccumulatedValues = (E::G1, E::G2);

    fn add_pairing_check(&mut self, accumulated_values: &(E::G1, E::G2)) {
        self.g1s.push(accumulated_values.0);
        self.g2s.push(accumulated_values.1);
    }

    fn check_pairings(&self) -> bool {
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
            .map(|(g1, g2)| (g1, g2))
            .collect::<Vec<_>>();

        let gt_result = E::multi_miller_loop(g1g2_pairs.as_slice());
        gt_result.final_exponentiation().is_identity().into()
    }
}
