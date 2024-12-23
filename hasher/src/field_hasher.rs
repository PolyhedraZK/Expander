use std::{fmt::Debug, io::Cursor};

use arith::{ExtensionField, Field, FieldSerde};

/// FieldHasherState depicts the field hasher's state as the hasher taking in field element.
///
/// The current design has a few components involving, including the output digest field type,
/// state width, namely how many field elements needed to construct the field hasher state from
/// input field elements, a method to construct the state from input elems, and a digest method
/// to squeeze the output from the hash state.
pub trait FieldHasherState:
    Debug + Sized + Default + Clone + Copy + PartialEq + FieldSerde
{
    /// InputF type of the input field elements into the field hasher.
    type InputF: Field;

    /// Output type of the field hasher state, can be the extension field of F, or F itself.
    type OutputF: Field;

    /// STATE_WIDTH of the field hasher state, namely how many input field elements needed to
    /// construct the field hasher state.
    const STATE_WIDTH: usize;

    /// STATE_NAME, say what should we call this particular FieldHasherState instantiation.
    ///
    /// NOTE(HS) we actually want to call it NAME, but in cases like MiMC, the state can be
    /// a Fr, that will collide with the naming on Field trait side.
    const STATE_NAME: &'static str;

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
pub trait FieldHasher<State: FieldHasherState>: Default + Debug + Clone + PartialEq {
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
    fn permute(&self, state: &mut State);

    /// hash takes in a bunch of input field elements and spits out an output field element.
    fn hash(&self, input: &[State::InputF]) -> State::OutputF {
        let mut state = State::from_elems(input);
        self.permute(&mut state);
        state.digest()
    }

    /// hash_to_field is a generalized method that on given an arbitrary extension field,
    /// extract from the permuted hash state and cast teh hash result into an extension field.
    fn hash_to_field<ExtF: ExtensionField<BaseField = State::InputF>>(
        &self,
        input: &[State::InputF],
    ) -> ExtF {
        let mut state = State::from_elems(input);
        self.permute(&mut state);
        ExtF::from_limbs(&state.to_elems())
    }
}

/// FiatShamirSponge is the sponge hash function meant to be used in the Fiat-Shamir transcript.
///
/// The behavior is mainly absorb inputs and squeeze an output field element.
/// The behavior relies on the underlying HasherState and the Hasher.
pub trait FiatShamirSponge<State: FieldHasherState>: Default + Debug + Clone + PartialEq {
    /// NAME, what family of instances of sponge hash function should be called.
    const NAME: &'static str;

    /// new constructs a new instance of FiatShamirSponge.
    fn new() -> Self;

    /// update takes in a list of inputs and absorbs into internal sponge hasher state.
    ///
    /// NOTE: the expected behavior is, if there are at most STATE_WIDTH elements not absorbed into
    /// a digests, but they should be absorbed into a hash digest by the end of squeeze.
    fn update(&mut self, inputs: &[State::InputF]);

    /// squeeze forces to absorb all current hasher state and outputs a digest over OutputF.
    fn squeeze(&mut self) -> State::OutputF;

    /// is_squeezed checks if the sponge function has absorbed but not hashed elements.
    fn is_squeezed(&self) -> bool;

    /// state is the current sponge state of the sponge function.
    fn state(&self) -> State;

    /// state_mut is the mut reference to the current sponge state.
    fn state_mut(&mut self) -> &mut State;

    /// serialize_state serializes the current sponge state into u8 slice, typically used broadcast.
    fn serialize_state(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        self.state().serialize_into(&mut buffer).unwrap();
        buffer
    }

    /// deserialize_state_to_self sets the state against the serialized state, typically used in
    /// broadcast.
    fn deserialize_state_to_self(&mut self, state: &[u8]) {
        let buffer = state.to_vec();
        let mut cursor = Cursor::new(buffer);
        *self.state_mut() = State::deserialize_from(&mut cursor).unwrap();
    }
}
