package transcript

import (
	"ExpanderVerifierCircuit/modules/fields"

	poseidonM31 "github.com/PolyhedraZK/ExpanderCompilerCollection/circuit-std-go/poseidon-m31"
	"github.com/consensys/gnark/frontend"
)

type PoseidonM31x16Hasher struct {
	fields.ArithmeticEngine
}

func NewPoseidonM31x16Hasher(api fields.ArithmeticEngine) PoseidonM31x16Hasher {
	return PoseidonM31x16Hasher{ArithmeticEngine: api}
}

func (h *PoseidonM31x16Hasher) StateCapacity() uint {
	return 8
}

func (h *PoseidonM31x16Hasher) HashToState(fs ...frontend.Variable) ([]frontend.Variable, uint) {
	return poseidonM31.PoseidonM31x16HashToState(h.ArithmeticEngine.API, fs)
}
