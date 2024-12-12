package transcript

import (
	"fmt"

	"ExpanderVerifierCircuit/modules/fields"

	"github.com/consensys/gnark/frontend"
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

// NewTranscript is the enter point to construct a new instance of transcript,
// that is decided by the field element tied to the transcript
func NewTranscript(
	api frontend.API,
	fieldEnum fields.ECCFieldEnum,
) (Transcript, error) {
	switch fieldEnum {
	case fields.ECCBN254:
		return NewMiMCTranscript(api)
	case fields.ECCM31:
		// TODO(HS) finish Poseidon hash based transcript ...
		fallthrough
	default:
		return nil,
			fmt.Errorf("unsupported transcript from field type %d", fieldEnum)
	}
}
