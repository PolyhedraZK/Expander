/// A column can be a, b, or c
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColumnID {
    A,    // witness A
    B,    // witness B
    C,    // witness C
    NONE, // no witness
}

impl Default for ColumnID {
    fn default() -> Self {
        ColumnID::NONE
    }
}

/// the index of the row
pub type RowID = usize;

pub struct Cell {
    pub(crate) column_id: ColumnID,
    pub(crate) row: RowID,
}


pub struct PermutationMap {
    /// a map from the original cell to the new cell
    /// indicates the two cells contain a same variable
    pub(crate) map: Vec<(Cell, Cell)>,
}