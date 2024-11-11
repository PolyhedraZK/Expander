package circuit

import (
	"math/big"

	"github.com/consensys/gnark/frontend"
)

func NewRandomLayer(input_len_log uint, output_len_log uint, public_input_start_idx *uint) *Layer {
	var layer = Layer{}

	layer.InputLenLog = input_len_log
	layer.OutputLenLog = output_len_log

	var input_size = uint(1) << input_len_log
	var output_size = uint(1) << output_len_log
	for i := uint(0); i < output_size; i++ {
		layer.Add = append(layer.Add,
			Gate{
				IIds: []uint{i % input_size},
				OId:  i,
				Coef: Coef{Constant, *big.NewInt(1), 0, 0},
			},
		)

		layer.Mul = append(layer.Mul,
			Gate{
				IIds: []uint{i % input_size, (i * 2) % input_size},
				OId:  i,
				Coef: Coef{Constant, *big.NewInt(1), 0, 0},
			},
		)

		layer.Cst = append(layer.Cst,
			Gate{
				IIds: make([]uint, 0),
				OId:  i,
				Coef: Coef{PublicInput, *big.NewInt(0), 0, *public_input_start_idx},
			},
		)
		(*public_input_start_idx)++
	}

	return &layer
}

func NewRandomCircuit(n_layers uint, simd_size uint, mpi_size uint, set_public_input bool) *Circuit {
	var circuit = Circuit{}

	var n_public_input uint = 0
	for i := uint(0); i < n_layers; i++ {
		circuit.Layers = append(circuit.Layers, *NewRandomLayer(
			n_layers-i+1,
			n_layers-i,
			&n_public_input,
		))
	}

	for i := uint(0); i < mpi_size*simd_size; i++ {
		circuit.PublicInput = append(circuit.PublicInput, make([]frontend.Variable, n_public_input))
		if set_public_input {
			for j := uint(0); j < n_public_input; j++ {
				circuit.PublicInput[i][j] = 0
			}
		}
	}

	circuit.ExpectedNumOutputZeros = uint(1) << circuit.Layers[len(circuit.Layers)-1].OutputLenLog
	return &circuit
}
