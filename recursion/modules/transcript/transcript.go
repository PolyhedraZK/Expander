package transcript

import (
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/std/hash"
	"github.com/consensys/gnark/std/hash/mimc"
)

type Transcript struct {
	api frontend.API

	// The hash function
	hasher hash.FieldHasher

	// The values to feed the hash function
	t []frontend.Variable

	// The state
	state frontend.Variable

	// helper field: counting, irrelevant to circuit
	count uint
}

func NewTranscript(api frontend.API) (Transcript, error) {
	mimc, err := mimc.NewMiMC(api)
	T := Transcript{
		api:    api,
		t:      []frontend.Variable{},
		hasher: &mimc,
		state:  0,
		count:  0,
	}

	return T, err
}

func (T *Transcript) AppendF(f frontend.Variable) {
	T.count++
	T.t = append(T.t, f)
}

func (T *Transcript) ChallengeF() frontend.Variable {
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

func (T *Transcript) ChallengeFs(n uint) []frontend.Variable {
	cs := make([]frontend.Variable, n)
	for i := uint(0); i < n; i++ {
		cs[i] = T.ChallengeF()
	}
	return cs
}

func (T *Transcript) GetState() frontend.Variable {
	return T.state
}

func (T *Transcript) GetCount() uint {
	return T.count
}

func (T *Transcript) ResetCount() {
	T.count = 0
}
