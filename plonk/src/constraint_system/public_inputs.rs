pub struct PublicInputs<F> {
    /// the public inputs
    pub public_inputs: Vec<F>,
    /// row index: the index of the row that the public input belongs to
    pub row_index: Vec<usize>,
}
