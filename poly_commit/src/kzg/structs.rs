use halo2curves::pairing::Engine;

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

/// Univariate KZG PCS verifier's params.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct UniKZGVerifierParams<E: Engine> {
    /// \tau over G2
    pub tau_g2: E::G2Affine,
}

impl<E: Engine> From<&CoefFormUniKZGSRS<E>> for UniKZGVerifierParams<E> {
    fn from(value: &CoefFormUniKZGSRS<E>) -> Self {
        Self {
            tau_g2: value.tau_g2,
        }
    }
}

#[derive(Debug, Default)]
pub struct HyperKZGOpening<E: Engine> {
    pub folded_oracle_commitments: Vec<E::G1>,
    pub f_beta2: E::Fr,
    pub evals_at_beta: Vec<E::Fr>,
    pub evals_at_neg_beta: Vec<E::Fr>,
    pub beta_commitment: E::G1,
    pub tau_vanishing_commitment: E::G1,
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
