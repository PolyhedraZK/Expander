use halo2curves::pairing::Engine;

/// Structured reference string for Bi-KZG polynomial commitment scheme.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CoefFormBiKZGSRS<E: Engine> {
    /// (g_1^{\tau_0^i\tau_1^j})_{i\in [0,N], j\in [0, M]} = \\
    /// (
    ///  g_1, g_1^{\tau_0}, g_1^{\tau_0^2}, ..., g_1^{\tau_0^N},
    ///  g_1^{\tau_1}, g_1^{\tau_0\tau_1}, g_1^{\tau_0^2\tau_1}, ..., g_1^{\tau_0^N\tau_1},
    ///  ..., g_1^{\tau_0^N\tau_1^M}
    /// )
    pub powers_of_g: Vec<E::G1Affine>,
    /// g in lagrange form over omega_0 and omega_1
    pub powers_of_g_lagrange_over_both_roots: Vec<E::G1Affine>,
    /// The generator of G2.
    pub h: E::G2Affine,
    /// tau_0 times the above generator of G2.
    pub tau_0_h: E::G2Affine,
    /// tau_1 times the above generator of G2.
    pub tau_1_h: E::G2Affine,
}

/// Structured reference string for Bi-KZG polynomial commitment scheme.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LagrangeFormBiKZGSRS<E: Engine> {
    /// The generator of G1
    pub g: E::G1Affine,
    /// g in lagrange form over omega_0
    pub powers_of_g_lagrange_over_x: Vec<E::G1Affine>,
    /// g in lagrange form over omega_0 and omega_1
    pub powers_of_g_lagrange_over_both_roots: Vec<E::G1Affine>,
    /// The generator of G2.
    pub h: E::G2Affine,
    /// tau_0 times the above generator of G2.
    pub tau_0_h: E::G2Affine,
    /// tau_1 times the above generator of G2.
    pub tau_1_h: E::G2Affine,
}

/// `UnivariateVerifierParam` is used to check evaluation proofs for a given
/// commitment.
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct BiKZGVerifierParam<E: Engine> {
    /// The generator of G1.
    pub g: E::G1Affine,
    /// The generator of G2.
    pub h: E::G2Affine,
    /// tau_0 times the above generator of G2.
    pub tau_0_h: E::G2Affine,
    /// tau_1 times the above generator of G2.
    pub tau_1_h: E::G2Affine,
}

/// Commitment Bi-KZG polynomial commitment scheme.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BiKZGCommitment<E: Engine> {
    /// the actual commitment is an affine point.
    pub com: E::G1Affine,
}

/// Proof for Bi-KZG polynomial commitment scheme.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BiKZGProof<E: Engine> {
    /// the actual proof is a pair of affine points.
    pub pi0: E::G1Affine,
    pub pi1: E::G1Affine,
}

impl<E: Engine> From<&CoefFormBiKZGSRS<E>> for BiKZGVerifierParam<E> {
    fn from(srs: &CoefFormBiKZGSRS<E>) -> Self {
        Self {
            g: srs.powers_of_g[0],
            h: srs.h,
            tau_0_h: srs.tau_0_h,
            tau_1_h: srs.tau_1_h,
        }
    }
}

impl<E: Engine> From<&LagrangeFormBiKZGSRS<E>> for BiKZGVerifierParam<E> {
    fn from(srs: &LagrangeFormBiKZGSRS<E>) -> Self {
        Self {
            g: srs.g,
            h: srs.h,
            tau_0_h: srs.tau_0_h,
            tau_1_h: srs.tau_1_h,
        }
    }
}
