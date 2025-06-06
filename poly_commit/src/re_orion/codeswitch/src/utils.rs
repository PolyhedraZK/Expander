use p3_field::Algebra;

pub fn mul5<Expr: Algebra<Expr>>(x: Expr) -> Expr {
    x.double().double() + x
}