use halo2curves::pairing::Engine;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UniKZGSRS<E: Engine> {
    pub g: Vec<E::G1Affine>,
    pub g_lagrange: Vec<E::G1Affine>,
    pub g2: E::G2Affine,
    pub s_g2: E::G2Affine,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UniVerifierParam<E: Engine> {
    pub s_g2: E::G2Affine,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UniKZGCommitment<E: Engine> {
    pub commitment: E::G1Affine,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UniKZGOepning<E: Engine> {
    pub opening: E::G1Affine,
}

impl<E: Engine> From<&UniKZGSRS<E>> for UniVerifierParam<E> {
    fn from(srs: &UniKZGSRS<E>) -> Self {
        Self { s_g2: srs.s_g2 }
    }
}
