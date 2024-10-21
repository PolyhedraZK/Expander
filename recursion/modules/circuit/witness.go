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

func (w *Witness) ToPubPri() ([][]frontend.Variable, [][]frontend.Variable) {
	public_input := make([][]frontend.Variable, w.NumWitnesses)
	private_input := make([][]frontend.Variable, w.NumWitnesses)

	witness_size := w.NumPrivateInputsPerWitness + w.NumPublicInputsPerWitness
	for i := uint(0); i < w.NumWitnesses; i++ {
		start_idx := i * witness_size
		private_input[i] = make([]frontend.Variable, 0)
		for j := uint(0); j < w.NumPrivateInputsPerWitness; j++ {
			private_input[i] = append(private_input[i], w.Values[start_idx+j])
		}

		start_idx += w.NumPrivateInputsPerWitness
		public_input[i] = make([]frontend.Variable, 0)
		for j := uint(0); j < w.NumPublicInputsPerWitness; j++ {
			public_input[i] = append(public_input[i], w.Values[start_idx+j])
		}
	}

	return public_input, private_input
}
