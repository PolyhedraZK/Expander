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

	// helper field: an index pointing into the hash state for next unconsumed
	// base field element
	nextUnconsumed uint

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
	}
}

func (t *FieldHasherTranscript) AppendF(f frontend.Variable) {
	t.dataPool = append(t.dataPool, f)
}

func (t *FieldHasherTranscript) AppendFs(fs ...frontend.Variable) {
	t.dataPool = append(t.dataPool, fs...)
}

func (t *FieldHasherTranscript) CircuitF() frontend.Variable {
	if len(t.dataPool) != 0 {
		var newCount uint = 0
		t.hashState, newCount = t.hasher.HashToState(t.dataPool...)

		t.count += newCount
		t.nextUnconsumed = 0
		t.dataPool = nil
	}

	if t.nextUnconsumed+1 <= t.hasher.StateCapacity() {
		res := t.hashState[t.nextUnconsumed]
		t.nextUnconsumed++
		return res
	} else {
		var newCount uint = 0
		t.hashState, newCount = t.hasher.HashToState(t.hashState...)
		t.count += newCount

		res := t.hashState[0]
		t.nextUnconsumed = 1

		return res
	}
}

func (t *FieldHasherTranscript) ChallengeF() []frontend.Variable {
	if len(t.dataPool) != 0 {
		var newCount uint = 0
		t.hashState, newCount = t.hasher.HashToState(t.dataPool...)

		t.count += newCount
		t.nextUnconsumed = 0
		t.dataPool = nil
	}

	if t.ChallengeFieldDegree()+t.nextUnconsumed <= t.hasher.StateCapacity() {
		sampledChallenge := make([]frontend.Variable, t.ChallengeFieldDegree())

		sliceStart := t.nextUnconsumed
		sliceEnd := t.nextUnconsumed + t.ChallengeFieldDegree()

		copy(sampledChallenge, t.hashState[sliceStart:sliceEnd])
		t.nextUnconsumed += t.ChallengeFieldDegree()

		return sampledChallenge
	}

	var sampledChallenge []frontend.Variable = nil
	if t.nextUnconsumed < t.hasher.StateCapacity() {
		sampledChallenge = append(
			sampledChallenge,
			t.hashState[t.nextUnconsumed:t.hasher.StateCapacity()]...,
		)
	}
	remainingElems := t.ChallengeFieldDegree() - uint(len(sampledChallenge))

	var newCount uint = 0
	t.hashState, newCount = t.hasher.HashToState(t.hashState...)
	t.count += newCount
	sampledChallenge = append(sampledChallenge, t.hashState[:remainingElems]...)
	t.nextUnconsumed = remainingElems

	return sampledChallenge
}

func (t *FieldHasherTranscript) HashAndReturnState() []frontend.Variable {
	var newCount uint = 0

	if len(t.dataPool) != 0 {
		t.hashState, newCount = t.hasher.HashToState(t.dataPool...)

		t.count += newCount
		t.dataPool = nil
	} else {
		t.hashState, newCount = t.hasher.HashToState(t.hashState...)

		t.count += newCount
	}

	return t.hashState
}

func (t *FieldHasherTranscript) SetState(newHashState []frontend.Variable) {
	t.nextUnconsumed = t.hasher.StateCapacity()
	t.hashState = newHashState
}

func (t *FieldHasherTranscript) GetCount() uint {
	return t.count
}

func (t *FieldHasherTranscript) ResetCount() {
	t.count = 0
}
