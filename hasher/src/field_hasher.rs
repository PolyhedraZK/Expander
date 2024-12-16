use std::fmt::Debug;

use arith::{ExtensionField, Field};

/// FieldHasherState depicts the field hasher's state as the hasher taking in field element.
///
/// The current design has a few components involving, including the output digest field type,
/// state width, namely how many field elements needed to construct the field hasher state from
/// input field elements, a method to construct the state from input elems, and a digest method
/// to squeeze the output from the hash state.
pub trait FieldHasherState: Debug + Sized + Default + Clone + Copy + PartialEq {
    /// InputF type of the input field elements into the field hasher.
    type InputF: Field;

    /// Output type of the field hasher state, can be the extension field of F, or F itself.
    type OutputF: Field;

    /// STATE_WIDTH of the field hasher state, namely how many input field elements needed to
    /// construct the field hasher state.
    const STATE_WIDTH: usize;

    /// NAME, say what should we call this particular FieldHasherState instantiation.
    const NAME: &'static str;

    /// OUTPUT_ELEM_DEG assumes output element is an extension field of input element,
    /// then this constant computes over the ratio of output field element size over the field size
    /// of the input field ones.
    const OUTPUT_ELEM_DEG: usize = Self::OutputF::FIELD_SIZE / Self::InputF::FIELD_SIZE;

    /// from_elems constructs an instance of field hasher state from the input field elements.
    fn from_elems(elems: &[Self::InputF]) -> Self;

    /// to_elems is a conjugate method of from_elems, that separates the state into elems.
    fn to_elems(&self) -> Vec<Self::InputF>;

    /// digest method squeezes output field element from the current hash state.
    fn digest(&self) -> Self::OutputF;
}

/// FieldHasher depicts the behavior of a field hasher, that takes in a bunch of field elems
/// and spits out an output field element.
pub trait FieldHasher<HasherState: FieldHasherState>: Default + Debug + Clone {
    /// NAME, say what is this family of instances of field hasher called
    const NAME: &'static str;

    /// new constructs a new instance of FieldHasher.
    fn new() -> Self;

    /// permute is the method that FieldHasher needs to implement.
    ///
    /// Using the info carried by FieldHasher instantiator, namely a struct,
    /// FieldHasher perform hash operation over the HashState instance.
    ///
    /// Note that this naming derive from the Poseidon hash permutation,
    /// should be applied in analogy to other context like MiMC.
    fn permute(&self, state: &mut HasherState);

    /// hash takes in a bunch of input field elements and spits out an output field element.
    fn hash(&self, input: &[HasherState::InputF]) -> HasherState::OutputF {
        let mut state = HasherState::from_elems(input);
        self.permute(&mut state);
        state.digest()
    }

    /// hash_to_field is a generalized method that on given an arbitrary extension field,
    /// extract from the permuted hash state and cast teh hash result into an extension field.
    fn hash_to_field<ExtF: ExtensionField<BaseField = HasherState::InputF>>(
        &self,
        input: &[HasherState::InputF],
    ) -> ExtF {
        let mut state = HasherState::from_elems(input);
        self.permute(&mut state);
        ExtF::from_limbs(&state.to_elems())
    }
}

/// FieldHasherSponge is the sponge hash function meant to be used in the Fiat-Shamir transcript.
///
/// The behavior is mainly absorb inputs and squeeze an output field element.
/// The behavior relies on the underlying HasherState and the Hasher.
pub trait FieldHasherSponge<State: FieldHasherState, Hasher: FieldHasher<State>>:
    Default + Debug + Clone
{
    /// NAME, what family of instances of sponge hash function should be called.
    const NAME: &'static str;

    /// new constructs a new instance of FieldHasherSponge.
    fn new() -> Self;

    /// update takes in a list of inputs and absorbs into internal sponge hasher state.
    ///
    /// NOTE: the expected behavior is, if there are at most STATE_WIDTH elements not absorbed into
    /// a digests, but they should be absorbed into a hash digest by the end of squeeze.
    fn update(&mut self, inputs: &[State::InputF]);

    /// squeeze forces to absorb all current hasher state and outputs a digest over OutputF.
    fn squeeze(&mut self) -> State::OutputF;
}
