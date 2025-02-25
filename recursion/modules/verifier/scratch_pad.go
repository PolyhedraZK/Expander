package verifier

import (
	"ExpanderVerifierCircuit/modules/circuit"
	"ExpanderVerifierCircuit/modules/fields"

	"github.com/consensys/gnark/frontend"
)

type ScratchPad struct {
	// ====== for evaluating cst, add and mul ======
	EqEvalsAtRz0   [][]frontend.Variable
	EqEvalsAtRz1   [][]frontend.Variable
	EqEvalsAtRSimd [][]frontend.Variable
	EqEvalsAtRMpi  [][]frontend.Variable

	EqEvalsAtRx [][]frontend.Variable
	EqEvalsAtRy [][]frontend.Variable

	EqEvalsFirstPart  [][]frontend.Variable
	EqEvalsSecondPart [][]frontend.Variable

	RSimd          [][]frontend.Variable
	RMpi           [][]frontend.Variable
	EqRSimdRSimdXY []frontend.Variable
	EqRMpiRMpiXY   []frontend.Variable

	// ====== for deg2, deg3 eval ======
	Inv2             frontend.Variable
	Deg3EvalAt       [4]frontend.Variable
	Deg3LagDenomsInv [4]frontend.Variable

	// ====== helper field to get the statistics of the circuit =====
	EqEvalsCount map[uint]uint
}

func NewScratchPad(
	api fields.ArithmeticEngine,
	circuit *circuit.Circuit,
	mpiSize uint,
) ScratchPad {
	maxNumVars := uint(0)
	for _, layer := range circuit.Layers {
		maxNumVars = max(maxNumVars, layer.InputLenLog, layer.OutputLenLog)
	}
	maxIOSize := uint(1) << maxNumVars

	sp := ScratchPad{
		EqEvalsAtRz0:   api.Zeroes(maxIOSize),
		EqEvalsAtRz1:   api.Zeroes(maxIOSize),
		EqEvalsAtRSimd: api.Zeroes(api.SIMDPackSize()),
		EqEvalsAtRMpi:  api.Zeroes(mpiSize),

		EqEvalsAtRx: api.Zeroes(maxIOSize),
		EqEvalsAtRy: api.Zeroes(maxIOSize),

		EqEvalsFirstPart:  api.Zeroes(maxIOSize),
		EqEvalsSecondPart: api.Zeroes(maxIOSize),

		Inv2:       api.Inverse(2),
		Deg3EvalAt: [4]frontend.Variable{0, 1, 2, 3},

		EqEvalsCount: make(map[uint]uint),
	}

	for i := 0; i < 4; i++ {
		var denominator frontend.Variable = 1
		for j := 0; j < 4; j++ {
			if j == i {
				continue
			}
			denominator = api.Mul(
				denominator,
				api.Sub(sp.Deg3EvalAt[i], sp.Deg3EvalAt[j]),
			)
		}
		sp.Deg3LagDenomsInv[i] = api.Inverse(denominator)
	}

	return sp
}
