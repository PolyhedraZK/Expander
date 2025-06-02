use p3_field::Field as P3Field;

pub fn mul_base<F: P3Field>(lhs: &[F], rhs: &F, res: &mut [F]) {
    for (i, &l) in lhs.iter().enumerate() {
        res[i] = F * *rhs;
    }
}