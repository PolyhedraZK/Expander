use arith::Field;
// use mersenne31::M31;
use p3_field::{Algebra, Field as P3Field};

pub fn mul5<Expr: Algebra<Expr>>(x: Expr) -> Expr {
    x.double().double() + x
}