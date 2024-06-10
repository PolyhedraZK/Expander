use ark_std::test_rng;
use halo2curves::bn256::{Bn256, Fr};

use crate::{bi_kzg::BiKZG, pcs::PolynomialCommitmentScheme, BivaraitePolynomial};

#[test]
fn test_bi_kzg_e2e() {
    let mut rng = test_rng();
    let n = 2;
    let m = 4;
    let srs = BiKZG::<Bn256>::gen_srs_for_testing(&mut rng, n, m);

    let poly = BivaraitePolynomial::new(
        vec![
            Fr::from(1u64),
            Fr::from(2u64),
            Fr::from(3u64),
            Fr::from(4u64),
            Fr::from(5u64),
            Fr::from(6u64),
            Fr::from(7u64),
            Fr::from(8u64),
        ],
        n,
        m,
    );
    // let poly = BivaraitePolynomial::random(&mut rng, n, m);

    let x = Fr::from(5u64);
    let y = Fr::from(6u64);

    let commit = BiKZG::<Bn256>::commit(&srs, &poly);
    let (proof, eval) = BiKZG::<Bn256>::open(&srs, &poly, &(x, y));

    assert!(BiKZG::<Bn256>::verify(
        &srs.into(),
        &commit,
        &(x, y),
        &eval,
        &proof
    ));

    assert!(false)
}
