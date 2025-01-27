package circuit

const VERSION_NUM uint = 3914834606642317635 // b'CIRCUIT6'

// LEADING_FIELD_BYTES is the first 32 bytes in the beginning of
// the circuit/witness file, standing for the modulus of the field that
// the circuit runs over, tied to the GKR proof
const LEADING_FIELD_BYTES uint = 32

// RAW_COMMITMENT_LENGTH_BYTES is the length, 32 bytes, to record the
// length of raw commitment.  The design was intended to allow for both
// BN254 and M31 modulus reading the proof bytes in their own units of
// field bytes.
const RAW_COMMITMENT_LENGTH_BYTES uint = 32
