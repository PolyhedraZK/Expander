package transcript

import (
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/std/hash/mimc"
)

func NewMiMCTranscript(api frontend.API) (*FieldHasherTranscript, error) {
	mimc, err := mimc.NewMiMC(api)
	T := FieldHasherTranscript{
		api:    api,
		t:      []frontend.Variable{},
		hasher: &mimc,
	}

	return &T, err
}

func (T *FieldHasherTranscript) AppendF(f frontend.Variable) {
	T.count++
	T.t = append(T.t, f)
}

func (T *FieldHasherTranscript) AppendFs(fs ...frontend.Variable) {
	for _, f := range fs {
		T.AppendF(f)
	}
}

func (T *FieldHasherTranscript) ChallengeF() frontend.Variable {
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

func (T *FieldHasherTranscript) ChallengeFs(n uint) []frontend.Variable {
	cs := make([]frontend.Variable, n)
	for i := uint(0); i < n; i++ {
		cs[i] = T.ChallengeF()
	}
	return cs
}

func (T *FieldHasherTranscript) GetState() frontend.Variable {
	return T.state
}

func (T *FieldHasherTranscript) GetCount() uint {
	return T.count
}

func (T *FieldHasherTranscript) ResetCount() {
	T.count = 0
}
