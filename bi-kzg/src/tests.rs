use ark_std::test_rng;
use halo2curves::bn256::Bn256;

use crate::{bi_kzg::BiKZG, pcs::PolynomialCommitmentScheme};

#[test]
fn test_bi_kzg_e2e() {
    let mut rng = test_rng();
    let n = 4;
    let m = 8;
    let srs = BiKZG::<Bn256>::gen_srs_for_testing(&mut rng, n, m);

    assert!(false)
}
