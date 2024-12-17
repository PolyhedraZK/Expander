package transcript

import (
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/std/hash/mimc"
)

// PoseidonTranscript is a direct field embedding from FieldHasherTranscript,
// that directly use the fields inside of a FieldHasherTranscript instance.
type PoseidonTranscript struct {
	FieldHasherTranscript
}

// NewPoseidonTranscript constructs a new PoseidonTranscript instance
func NewPoseidonTranscript(api frontend.API) (*PoseidonTranscript, error) {
	// TODO(HS) poseidon based transcript
	mimc, err := mimc.NewMiMC(api)
	fsT := FieldHasherTranscript{
		api:    api,
		t:      []frontend.Variable{},
		hasher: &mimc,
	}

	return &PoseidonTranscript{FieldHasherTranscript: fsT}, err
}

func (T *PoseidonTranscript) AppendF(f frontend.Variable) {
	T.count++
	T.t = append(T.t, f)
}

func (T *PoseidonTranscript) AppendFs(fs ...frontend.Variable) {
	for _, f := range fs {
		T.AppendF(f)
	}
}

func (T *PoseidonTranscript) ChallengeF() frontend.Variable {
	T.hasher.Reset()
	if len(T.t) > 0 {
		for i := 0; i < len(T.t); i++ {
			T.hasher.Write(T.t[i])
		}
		T.t = T.t[:0]
	} else {
		T.hasher.Write(T.state)
		T.count++
	}
	T.state = T.hasher.Sum()
	return T.state
}

func (T *PoseidonTranscript) ChallengeFs(n uint) []frontend.Variable {
	cs := make([]frontend.Variable, n)
	for i := uint(0); i < n; i++ {
		cs[i] = T.ChallengeF()
	}
	return cs
}

func (T *PoseidonTranscript) GetState() frontend.Variable {
	return T.api
}

func (T *PoseidonTranscript) GetCount() uint {
	return T.count
}

func (T *PoseidonTranscript) ResetCount() {
	T.count = 0
}
