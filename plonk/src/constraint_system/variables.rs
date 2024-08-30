use arith::Field;

use super::ColumnID;

/// The position of the variable in the variable list
pub type Variable = usize;
/// Zero is always at index 0
pub const VAR_ZERO: Variable = 0;
/// One is always at index 1
pub const VAR_ONE: Variable = 1;

/// This struct stores all the variables that will be used in the circuit
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WitnessList<F> {
    // we store all the witnesses in the witnesses vector,
    // each individual witness is indexed by the variable index
    pub witnesses: Vec<F>,
}

impl<F: Field> WitnessList<F> {
    /// initialize a new variables struct
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// create a new witness for the element F
    /// return the index of the witness
    pub(crate) fn new_witness(&mut self, f: F) -> Variable {
        let index = self.witnesses.len();
        self.witnesses.push(f);
        index
    }

    /// get the index for zero
    pub(crate) fn zero(&self) -> Variable {
        // zero is always at index 0 by cs initialization
        VAR_ZERO
    }

    /// get the index for one
    pub(crate) fn one(&self) -> Variable {
        // one is always at index 1 by cs initialization
        VAR_ONE
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct VariableColumn {
    pub(crate) variables: Vec<Variable>,
    pub(crate) id: ColumnID,
}

impl VariableColumn {
    pub(crate) fn push(&mut self, var: Variable) {
        self.variables.push(var);
    }

    pub(crate) fn len(&self) -> usize {
        self.variables.len()
    }

    pub(crate) fn resize(&mut self, len: usize, index: Variable) {
        self.variables.resize(len, index);
    }

    pub(crate) fn get_var_index(&self, location: usize) -> Variable {
        self.variables[location]
    }
}
