use ark_std::test_rng;

use crate::field::Field;
use crate::ExtensionField;

pub(crate) fn random_extension_field_tests<F: ExtensionField>(_name: String) {
    let mut rng = test_rng();
    for _ in 0..1000 {
        {
            let a = F::random_unsafe(&mut rng);
            let s1 = F::BaseField::random_unsafe(&mut rng);
            let s2 = F::BaseField::random_unsafe(&mut rng);

            assert_eq!(
                a.mul_by_base_field(&s1).mul_by_base_field(&s2),
                a.mul_by_base_field(&s2).mul_by_base_field(&s1),
            );
            assert_eq!(
                a.mul_by_base_field(&s1).mul_by_base_field(&s2),
                a.mul_by_base_field(&(s1 * s2)),
            );

            assert_eq!(
                a.add_by_base_field(&s1).add_by_base_field(&s2),
                a.add_by_base_field(&s2).add_by_base_field(&s1),
            );
            assert_eq!(
                a.add_by_base_field(&s1).add_by_base_field(&s2),
                a.add_by_base_field(&(s1 + s2)),
            );
        }

        {
            let a = F::random_unsafe(&mut rng);
            let b = F::random_unsafe(&mut rng);
            let s = F::BaseField::random_unsafe(&mut rng);

            assert_eq!(a.mul_by_base_field(&s) * b, (a * b).mul_by_base_field(&s),);
            assert_eq!(b.mul_by_base_field(&s) * a, (a * b).mul_by_base_field(&s),);

            assert_eq!(a.add_by_base_field(&s) + b, (a + b).add_by_base_field(&s),);
            assert_eq!(b.add_by_base_field(&s) + a, (a + b).add_by_base_field(&s),);
        }

        {
            let a = F::random_unsafe(&mut rng);
            let b = F::X;
            let ax = a.mul_by_x();
            let ab = a * b;
            assert_eq!(ax, ab);
        }
    }
}
