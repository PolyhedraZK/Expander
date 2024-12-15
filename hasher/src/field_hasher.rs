use std::fmt::Debug;

use arith::Field;

/// FieldHasherState depicts the field hasher's state as the hasher taking in field element.
///
/// The current design has a few components involving, including the output digest field type,
/// state width, namely how many field elements needed to construct the field hasher state from
/// input field elements, a method to construct the state from input elems, and a digest method
/// to squeeze the output from the hash state.
pub trait FieldHasherState: Sized + Default + Clone + Copy + PartialEq {
    /// InputF type of the input field elements into the field hasher.
    type InputF;

    /// Output type of the field hasher state, can be the extension field of F, or F itself.
    type Output;

    /// STATE_WIDTH of the field hasher state, namely how many input field elements needed to
    /// construct the field hasher state.
    const STATE_WIDTH: usize;

    /// from_elems constructs an instance of field hasher state from the input field elements.
    fn from_elems(elems: &[Self::InputF]) -> Self;

    /// digest method squeezes output field element from the current hash state.
    fn digest(&self) -> Self::Output;
}

/// FieldHasher depicts the behavior of a field hasher, that takes in a bunch of field elems
/// and spits out an output field element.
pub trait FieldHasher<F: Field, OF: Field, HasherState: FieldHasherState<InputF = F, Output = OF>>:
    Default + Debug + Clone + Copy + PartialEq
{
    /// new constructs a new instance of FieldHasher.
    fn new() -> Self;

    /// hash takes in a bunch of input field elements and spits out an output field element.
    fn hash(&self, input: &[F]) -> OF;
}
