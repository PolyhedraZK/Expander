use arith::Fr;
use ark_std::rand::{thread_rng, Rng};
use halo2curves::{
    bn256::{G1Affine, G1},
    ff::Field,
    msm::best_multiexp,
};
use utils::small_scalar_msm::msm_serial_16bits;

#[test]
fn test_msm_serial_16bits() {
    let mut rng = thread_rng();

    for log_len in 2..16 {
        let len = 1 << log_len;
        let points = (0..len)
            .map(|_| G1Affine::random(&mut rng))
            .collect::<Vec<_>>();
        let scalars = (0..len)
            .map(|_| Fr::from(rng.gen::<u16>() as u64))
            .collect::<Vec<_>>();

        let mut result_16 = G1::default();
        msm_serial_16bits(&scalars, &points, &mut result_16);

        let result_naive = best_multiexp(&scalars, &points);

        assert_eq!(result_16, result_naive);
    }
}
