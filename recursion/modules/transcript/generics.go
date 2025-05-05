package transcript

import (
	"ExpanderVerifierCircuit/modules/fields"

	"github.com/consensys/gnark/frontend"
)

// FieldHasher interface align with the rust runtime FiatShamirFieldHasher trait
// to describe the behavior of a field hasher in Fiat-Shamir transcript.
// The implementation and instantiation should be considered to be immutable, as
// the functionality of sponge is managed by a FieldHasherTranscript instance.
type FieldHasher interface {
	// StateCapacity returns how many base field elements can be used in a state
	// dumped by the HashToState method.
	StateCapacity() uint

	// HashToState hashes a bunch of base field elements to a "hash state",
	// namely a slice of base field elements, can be used up to StateCapacity.
	HashToState(fs ...frontend.Variable) ([]frontend.Variable, uint)
}

// FieldHasherTranscript is the transcript constructed from field hasher, can be
// instantiated by MiMC or Poseidon hash for BN254 or Mersenne31
type FieldHasherTranscript struct {
	fields.ArithmeticEngine

	// The hash function
	hasher FieldHasher

	// The values to feed the hash function
	dataPool []frontend.Variable

	// The hashState
	hashState []frontend.Variable

	// helper field: counting, irrelevant to circuit
	count uint
}

// NewTranscript is the enter point to construct a new instance of transcript,
// that is decided by the field element tied to the transcript
func NewTranscript(arithmeticEngine fields.ArithmeticEngine) *FieldHasherTranscript {
	var hasher FieldHasher

	switch arithmeticEngine.ECCFieldEnum {
	case fields.ECCBN254:
		mimcHasher := NewMiMCFieldHasher(arithmeticEngine)
		hasher = &mimcHasher
	case fields.ECCM31:
		poseidonHasher := NewPoseidonM31x16Hasher(arithmeticEngine)
		hasher = &poseidonHasher
	case fields.ECCGF2:
		// NOTE(HS) for now we are not doing GF2 proof recursion
		fallthrough
	default:
		panic("unsupported transcript from field type")
	}

	return &FieldHasherTranscript{
		ArithmeticEngine: arithmeticEngine,
		hasher:           hasher,
		dataPool:         make([]frontend.Variable, 0),
		hashState: func() []frontend.Variable {
			initState := make([]frontend.Variable, hasher.StateCapacity())
			for i := range int(hasher.StateCapacity()) {
				initState[i] = 0
			}
			return initState
		}(),
		count: 0,
	}
}

func (t *FieldHasherTranscript) AppendF(f frontend.Variable) {
	t.dataPool = append(t.dataPool, f)
}

func (t *FieldHasherTranscript) AppendFs(fs ...frontend.Variable) {
	t.dataPool = append(t.dataPool, fs...)
}

func (t *FieldHasherTranscript) CircuitF() frontend.Variable {
	t.HashAndReturnState()
	return t.hashState[0]
}

func (t *FieldHasherTranscript) ChallengeF() []frontend.Variable {
	t.HashAndReturnState()
	return t.hashState[:t.ChallengeFieldDegree()]
}

func (t *FieldHasherTranscript) HashAndReturnState() []frontend.Variable {
	var newCount uint = 0

	if len(t.dataPool) != 0 {
		t.hashState, newCount = t.hasher.HashToState(append(t.hashState, t.dataPool...)...)

		t.count += newCount
		t.dataPool = nil
	} else {
		t.hashState, newCount = t.hasher.HashToState(t.hashState...)

		t.count += newCount
	}

	return t.hashState
}

func (t *FieldHasherTranscript) SetState(newHashState []frontend.Variable) {
	t.dataPool = nil
	t.hashState = newHashState
}

func (t *FieldHasherTranscript) GetCount() uint {
	return t.count
}

func (t *FieldHasherTranscript) ResetCount() {
	t.count = 0
}
