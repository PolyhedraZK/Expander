package circuit

import (
	"math/big"

	"github.com/consensys/gnark/frontend"
)

type Witness struct {
	NumWitnesses               uint
	NumPrivateInputsPerWitness uint
	NumPublicInputsPerWitness  uint
	Values                     []big.Int
}

// PubInput stores the circuit public inputs
type PubInput = []frontend.Variable

// PrivInput stores the circuit private inputs
type PrivInput = []frontend.Variable

// ToPubPrivInputs separate the Witness object into public inputs and private outputs
// Witness object stands for multiple instances of circuit inputs, and thus the
// output of public/private inputs are concatenations of each public/private inputs
func (w *Witness) ToPubPrivInputs() (pubInputs []PubInput, privInputs []PrivInput) {
	pubInputs = make([]PubInput, w.NumWitnesses)
	privInputs = make([]PrivInput, w.NumWitnesses)

	witnessSize := w.NumPrivateInputsPerWitness + w.NumPublicInputsPerWitness

	for ithWitness := uint(0); ithWitness < w.NumWitnesses; ithWitness++ {
		startIndex := ithWitness * witnessSize
		privInputs[ithWitness] = make([]frontend.Variable, w.NumPrivateInputsPerWitness)
		for i := uint(0); i < w.NumPrivateInputsPerWitness; i++ {
			privInputs[ithWitness][i] = w.Values[startIndex+i]
		}

		startIndex += w.NumPrivateInputsPerWitness
		pubInputs[ithWitness] = make([]frontend.Variable, w.NumPublicInputsPerWitness)
		for i := uint(0); i < w.NumPublicInputsPerWitness; i++ {
			pubInputs[ithWitness][i] = w.Values[startIndex+i]
		}
	}

	return
}
