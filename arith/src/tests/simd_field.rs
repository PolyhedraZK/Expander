use ark_std::test_rng;

use crate::field::Field;
use crate::SimdField;

pub(crate) fn random_simd_field_tests<F: SimdField>(_name: String) {
    let mut rng = test_rng();

    {
        let a = F::random_unsafe(&mut rng);
        let s1 = F::Scalar::random_unsafe(&mut rng);
        let s2 = F::Scalar::random_unsafe(&mut rng);

        assert_eq!(a.scale(&s1).scale(&s2), a.scale(&s2).scale(&s1),);
        assert_eq!(a.scale(&s1).scale(&s2), a.scale(&(s1 * s2)));
    }

    {
        let a = F::random_unsafe(&mut rng);
        let b = F::random_unsafe(&mut rng);
        let s = F::Scalar::random_unsafe(&mut rng);

        assert_eq!(a.scale(&s) * b, (a * b).scale(&s),);
        assert_eq!(b.scale(&s) * a, (a * b).scale(&s),);
    }
}
