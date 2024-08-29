use halo2curves::bn256::Fr;

use crate::FFTDomain;

#[test]
fn test_fft_domain_new() {
    let domain = FFTDomain::<Fr>::new(4);
    assert_eq!(domain.n, 16);
    assert_eq!(domain.log_n, 4);
}

#[test]
fn test_fft_ifft_roundtrip() {
    for log_n in 1..10 {
        let domain = FFTDomain::<Fr>::new(log_n);
        let mut input = (0..(1 << log_n))
            .map(|i| Fr::from(i as u64))
            .collect::<Vec<_>>();
        let original = input.clone();

        domain.fft_in_place(&mut input);
        domain.ifft_in_place(&mut input);

        for (a, b) in input.iter().zip(original.iter()) {
            assert_eq!(a, b);
        }
    }

    for log_n in 3..10 {
        let domain = FFTDomain::<Fr>::new(log_n);
        // a = x + 1
        let mut a = vec![Fr::zero(); 1 << log_n];
        a[0] = Fr::one();
        a[1] = Fr::one();
        // b = 2x + 1
        let mut b = vec![Fr::zero(); 1 << log_n];
        b[0] = Fr::one();
        b[1] = Fr::from(2u64);

        domain.fft_in_place(&mut a);
        domain.fft_in_place(&mut b);
        let mut c = a
            .iter()
            .zip(b.iter())
            .map(|(a, b)| a * b)
            .collect::<Vec<_>>();
        domain.ifft_in_place(&mut c);

        // c = 2x^2 + 3x + 1
        assert_eq!(c[0], Fr::from(1u64));
        assert_eq!(c[1], Fr::from(3u64));
        assert_eq!(c[2], Fr::from(2u64));
        for i in 3..(1 << log_n) {
            assert_eq!(c[i], Fr::zero());
        }
    }
}
