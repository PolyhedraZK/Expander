// /// Trait definitions for gates
// ///
// ///
// /// Vanilla plonk gate:
// ///
// ///     q_l * a + q_r * b + q_o * c + q_m * a * b + q_c = 0
// ///
// /// where
// /// - `a`, `b`, `c` are the variables of the constraint system.
// /// - `q_l`, `q_r`, `q_o`, `q_m` are the coefficients of the constraint system.
// /// - `q_c` is the constant term of the constraint system.
// ///
// /// Note that default implement all gates are zeros to ease the instantiation of the gates.
// pub trait Gate {
//     /// name
//     const NAME: &'static str;

//     /// add gate: a + b + c = 0
//     fn add_gate(&self, sum: &F) -> [F; 3] {
//         [F::zero(), F::zero(), F::zero()]
//     }

//     /// mul gate: a * b + c = 0
//     fn mul_gate(&self, product: &F) -> [F; 5] {
//         [F::zero(), F::zero()]
//     }
// }
