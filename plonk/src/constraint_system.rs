use arith::Field;

/// Constraint system for the vanilla plonk protocol.
///
/// Vanilla plonk gate:
///
///     q_l * a + q_r * b + q_o * c + q_m * a * b + q_c = 0
///
/// where
/// - `a`, `b`, `c` are the variables of the constraint system.
/// - `q_l`, `q_r`, `q_o`, `q_m` are the coefficients of the constraint system.
/// - `q_c` is the constant term of the constraint system.
///
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ConstraintSystem<F> {
    pub q_l: Vec<F>,
    pub q_r: Vec<F>,
    pub q_o: Vec<F>,
    pub q_m: Vec<F>,
    pub q_c: Vec<F>,

    pub a: Vec<F>,
    pub b: Vec<F>,
    pub c: Vec<F>,

    pub num_variables: usize,
}

impl<F: Field> ConstraintSystem<F> {
    /// Create a new, empty constraint system.
    pub fn new() -> Self {
        Self::default()
    }

    /// initialize a new constraint system with default constants
    pub fn init() -> Self {
        let mut cs = ConstraintSystem::new();
        cs.constant_gate(&F::zero());
        cs.constant_gate(&F::one());
        cs
    }

    /// constant gate
    pub fn constant_gate(&mut self, c: &F) {
        self.q_l.push(F::zero());
        self.q_r.push(F::zero());
        self.q_o.push(F::zero());
        self.q_m.push(F::zero());
        self.q_c.push(*c);

        self.a.push(F::zero());
        self.b.push(F::zero());
        self.c.push(F::zero());

        self.num_variables += 1;
    }

    /// addition gate
    pub fn addition_gate(&mut self, a: &F, b: &F) {}
}
