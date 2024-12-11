package circuit

const VERSION_NUM uint = 3914834606642317635 // b'CIRCUIT6'

// LEADING_FIELD_BYTES is the first 32 bytes in the beginning of
// the circuit/witness file, standing for the modulus of the field that
// the circuit runs over, tied to the GKR proof
const LEADING_FIELD_BYTES uint = 32
