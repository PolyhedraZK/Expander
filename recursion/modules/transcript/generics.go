package transcript

import (
	"fmt"

	"ExpanderVerifierCircuit/modules/fields"

	"github.com/consensys/gnark/frontend"
	gnarkHash "github.com/consensys/gnark/std/hash"
)

// Transcript interface describes the Fiat-Shamir transcript behaviors
type Transcript interface {
	// AppendF appends a field element into the transcript
	AppendF(f frontend.Variable)
	// AppendFs appends a list of field elements into the transcript
	AppendFs(fs ...frontend.Variable)
	// ChallengeF samples a field element out of the current transcript
	ChallengeF() frontend.Variable
	// ChallengeFs samples a list of field elements from the current Transcript
	ChallengeFs(uint) []frontend.Variable
	// GetState retrieves the current transcript hash state
	GetState() frontend.Variable
	// GetCount checks how many field elements pushed to the transcript
	GetCount() uint
	// ResetCount resets the transcript field elements count
	ResetCount()
}

// FieldHasherTranscript is the transcript constructed from field hasher, can be
// instantiated by MiMC or Poseidon hash for BN254 or Mersenne31
type FieldHasherTranscript struct {
	api frontend.API

	// The hash function
	hasher gnarkHash.FieldHasher

	// The values to feed the hash function
	t []frontend.Variable

	// The state
	state frontend.Variable

	// helper field: counting, irrelevant to circuit
	count uint
}

// NewTranscript is the enter point to construct a new instance of transcript,
// that is decided by the field element tied to the transcript
func NewTranscript(api frontend.API, fieldEnum fields.ECCFieldEnum) (Transcript, error) {
	switch fieldEnum {
	case fields.ECCBN254:
		return NewMiMCTranscript(api)
	case fields.ECCM31:
		// TODO(HS) finish Poseidon hash based transcript ...
		fallthrough
	case fields.ECCGF2:
		// TODO galois 2 transcript TBD
		fallthrough
	default:
		return nil,
			fmt.Errorf("unsupported transcript from field type %d", fieldEnum)
	}
}
