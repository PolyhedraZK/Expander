use halo2curves::ff::PrimeField;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PublicInputsIndices {
    /// row index: the index of the row that the public input belongs to
    pub row_index: Vec<usize>,
}

impl PublicInputsIndices {
    /// add a new public input
    pub(crate) fn push(&mut self, row_index: usize) {
        self.row_index.push(row_index);
    }

    /// convert to a vector of field elements
    pub fn build_pi_poly<F: PrimeField>(&self, public_inputs: &[F], n: usize) -> Vec<F> {
        assert!(
            public_inputs.len() == self.row_index.len(),
            "supplied public inputs ({}) does not match the number of public inputs in the cs ({})",
            public_inputs.len(),
            self.row_index.len()
        );
        let mut public_inputs_poly = vec![F::ZERO; n];
        for (i, &row_index) in self.row_index.iter().enumerate() {
            public_inputs_poly[row_index] = public_inputs[i];
        }
        public_inputs_poly
    }
}
