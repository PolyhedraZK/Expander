use arith::Field;

pub type VariableIndex = usize;

pub const VAR_ZERO: VariableIndex = 0;
pub const VAR_ONE: VariableIndex = 1;

/// This struct stores all the variables that will be used in the circuit
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Variables<F> {
    // we store all the witnesses in the witnesses vector,
    // each individual witness is indexed by the variable index
    pub variables: Vec<VariableIndex>,
    pub witnesses: Vec<F>,
}

impl<F: Field> Variables<F> {
    /// initialize a new variables struct
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// create a new variable for the element F
    pub(crate) fn new_variable(&mut self, f: F) -> VariableIndex {
        let index = self.variables.len();
        self.variables.push(index);
        self.witnesses.push(f);
        index
    }

    /// get the index for zero
    pub(crate) fn zero(&self) -> VariableIndex {
        // zero is always at index 0
        0
    }

    /// get the index for one
    pub(crate) fn one(&self) -> VariableIndex {
        // one is always at index 1
        1
    }
}

pub type VariableColumn = Vec<VariableIndex>;
