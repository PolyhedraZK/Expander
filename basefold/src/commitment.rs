use arith::{Field, FieldSerde};
use transcript::Transcript;
use tree::Tree;

#[derive(Debug, Clone, PartialEq)]
pub struct BasefoldCommitment<F: Field + FieldSerde> {
    pub(crate) tree: Tree<F>,
}

impl<F: Field + FieldSerde> BasefoldCommitment<F> {
    pub fn append_to_transcript<T>(&self, transcript: &mut T)
    where
        T: Transcript<F>,
    {
        transcript.append_u8_slice(self.tree.root().as_bytes());
    }
}
