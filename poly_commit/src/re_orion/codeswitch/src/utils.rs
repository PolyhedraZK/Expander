use arith::Field;
// use mersenne31::M31;
use p3_field::{Algebra, Field as P3Field};

// pub fn unit_mul<Expr: Algebra<Expr>>(lhs: &[Expr], rhs: &[Expr], res: &mut [Expr]) {
//     res[0] = lhs[0].clone() * rhs[0].clone()
// }

pub fn unit_mul<Expr: Algebra<Expr>>(lhs: &[Expr], rhs: &[Expr], res: &mut [Expr]) {
    res[0] = lhs[0].clone() * rhs[0].clone() + mul5(lhs[1].clone() * rhs[2].clone() + lhs[2].clone() * rhs[1].clone());
    res[1] = lhs[0].clone() * rhs[1].clone() + lhs[1].clone() * rhs[0].clone() + mul5(lhs[2].clone() * rhs[2].clone());
    res[2] = lhs[0].clone() * rhs[2].clone() + lhs[1].clone() * rhs[1].clone() + lhs[2].clone() * rhs[1].clone();
}

/*
struct Multiplier {}

trait Multiply<F: Field> {
    fn mul<Expr: Algebra<Expr>>(lhs: [Expr], rhs: [Expr], res: &mut [Expr]);
}

impl Multiply<M31> for Multiplier {
    #[inline(always)]
    fn mul<Expr: Algebra<Expr>>(lhs: [Expr], rhs: [Expr], res: &mut [Expr]) {
        res[0] = lhs[0] * rhs[0];
    }
}

impl Multiply<M31Ext> for Multiplier {
    #[inline(always)]
    fn mul<Expr: Algebra<Expr>>(lhs: [Expr], rhs: [Expr], res: &mut [Expr]) {
        res[0] = lhs[0] * rhs[0] + (lhs[1] * rhs[2] + lhs[2] * rhs[1]).mul_5();
        res[1] = lhs[0] * rhs[1] + lhs[1] * rhs[0] + (lhs[2] * rhs[2]).mul_5();
        res[2] = lhs[0] * rhs[2] + lhs[1] * rhs[1] + lhs[2] * rhs[1];
    }
} */

fn mul5<Expr: Algebra<Expr>>(x: Expr) -> Expr {
    x.double().double() + x
}