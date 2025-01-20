package transcript

import (
	"ExpanderVerifierCircuit/modules/fields"

	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/std/hash/mimc"
)

// MiMCFieldHasher is a wrapper around MiMC5 gnark hasher, that implements the
// FieldHasher interface.
type MiMCFieldHasher struct {
	mimc.MiMC
}

func NewMiMCFieldHasher(api fields.ArithmeticEngine) MiMCFieldHasher {
	mimc, err := mimc.NewMiMC(api)
	if err != nil {
		panic(err.Error())
	}

	return MiMCFieldHasher{MiMC: mimc}
}

func (m *MiMCFieldHasher) StateCapacity() uint {
	return 1
}

func (m *MiMCFieldHasher) HashToState(fs ...frontend.Variable) ([]frontend.Variable, uint) {
	m.MiMC.Reset()
	m.MiMC.Write(fs...)

	h0 := m.MiMC.Sum()
	hashCount := uint(len(fs))

	return []frontend.Variable{h0}, hashCount
}
