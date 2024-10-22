package verifier

import (
	"ExpanderVerifierCircuit/modules/circuit"

	"github.com/consensys/gnark/frontend"
)

type ScratchPad struct {
	// ====== for evaluating cst, add and mul ======
	EqEvalsAtRz0   []frontend.Variable
	EqEvalsAtRz1   []frontend.Variable
	EqEvalsAtRSimd []frontend.Variable
	EqEvalsAtRMpi  []frontend.Variable

	EqEvalsAtRx []frontend.Variable
	EqEvalsAtRy []frontend.Variable

	EqEvalsFirstPart  []frontend.Variable
	EqEvalsSecondPart []frontend.Variable

	RSimd          *[]frontend.Variable
	RMpi           *[]frontend.Variable
	EqRSimdRSimdXY frontend.Variable
	EqRMpiRMpiXY   frontend.Variable

	// ====== for deg2, deg3 eval ======
	Inv2             frontend.Variable
	Deg3EvalAt       [4]frontend.Variable
	Deg3LagDenomsInv [4]frontend.Variable
}

func NewScratchPad(api frontend.API, circuit *circuit.Circuit, simd_size uint, mpi_size uint) (*ScratchPad, error) {
	var sp = ScratchPad{}

	var max_num_var uint = 0
	for i := 0; i < len(circuit.Layers); i++ {
		var layer = circuit.Layers[i]
		max_num_var = max(max_num_var, layer.InputLenLog, layer.OutputLenLog)
	}
	var max_io_size uint = 1 << max_num_var

	sp.EqEvalsAtRz0 = make([]frontend.Variable, max_io_size)
	sp.EqEvalsAtRz1 = make([]frontend.Variable, max_io_size)
	sp.EqEvalsAtRSimd = make([]frontend.Variable, simd_size)
	sp.EqEvalsAtRMpi = make([]frontend.Variable, mpi_size)

	sp.EqEvalsAtRx = make([]frontend.Variable, max_io_size)
	sp.EqEvalsAtRy = make([]frontend.Variable, max_io_size)

	sp.EqEvalsFirstPart = make([]frontend.Variable, max_io_size)
	sp.EqEvalsSecondPart = make([]frontend.Variable, max_io_size)

	sp.Inv2 = api.Inverse(2)
	sp.Deg3EvalAt = [4]frontend.Variable{0, 1, 2, 3}
	for i := 0; i < 4; i++ {
		var denominator frontend.Variable = 1
		for j := 0; j < 4; j++ {
			if j == i {
				continue
			}
			denominator = api.Mul(denominator, api.Sub(sp.Deg3EvalAt[i], sp.Deg3EvalAt[j]))
		}
		sp.Deg3LagDenomsInv[i] = api.Inverse(denominator)
	}

	return &sp, nil
}
