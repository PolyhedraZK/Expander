package transcript

import (
	"ExpanderVerifierCircuit/modules/fields"

	"testing"

	"github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo"
	"github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo/test"
	"github.com/consensys/gnark/frontend"
	"github.com/stretchr/testify/require"
)

func TestPoseidonM31x16Params(t *testing.T) {
	require.Equal(t,
		uint(80596940),
		poseidonM31x16RoundConstant[0][0],
		"poseidon round constant m31x16 0.0 not matching ggs",
	)
}

type PoseidonM31x16FiatShamirHashCircuit struct {
	Inputs  []frontend.Variable
	Outputs []frontend.Variable
}

func NewPoseidonM31x16FiatShamirHashCircuit(inputLen uint) PoseidonM31x16FiatShamirHashCircuit {
	return PoseidonM31x16FiatShamirHashCircuit{
		Inputs:  make([]frontend.Variable, inputLen),
		Outputs: make([]frontend.Variable, 16),
	}
}

func (c *PoseidonM31x16FiatShamirHashCircuit) Define(api frontend.API) error {
	actualOut, _ := poseidonM31x16HashToState(api, c.Inputs)

	if len(actualOut) != len(c.Outputs) {
		panic("output length not matching")
	}

	for i := range actualOut {
		api.AssertIsEqual(actualOut[i], c.Outputs[i])
	}

	return nil
}

func TestPoseidonM31x16HashToState(t *testing.T) {

	testcases := []struct {
		InputLen   uint
		Assignment PoseidonM31x16FiatShamirHashCircuit
	}{
		{
			InputLen: 8,
			Assignment: PoseidonM31x16FiatShamirHashCircuit{
				Inputs: []frontend.Variable{
					114514, 114514, 114514, 114514,
					114514, 114514, 114514, 114514,
				},
				Outputs: []frontend.Variable{
					1021105124, 1342990709, 1593716396, 2100280498,
					330652568, 1371365483, 586650367, 345482939,
					849034538, 175601510, 1454280121, 1362077584,
					528171622, 187534772, 436020341, 1441052621,
				},
			},
		},
		{
			InputLen: 16,
			Assignment: PoseidonM31x16FiatShamirHashCircuit{
				Inputs: []frontend.Variable{
					114514, 114514, 114514, 114514,
					114514, 114514, 114514, 114514,
					114514, 114514, 114514, 114514,
					114514, 114514, 114514, 114514,
				},
				Outputs: []frontend.Variable{
					1510043913, 1840611937, 45881205, 1134797377,
					803058407, 1772167459, 846553905, 2143336151,
					300871060, 545838827, 1603101164, 396293243,
					502075988, 2067011878, 402134378, 535675968,
				},
			},
		},
	}

	for _, testcase := range testcases {
		circuit := NewPoseidonM31x16FiatShamirHashCircuit(testcase.InputLen)
		circuitCompileResult, err := ecgo.Compile(
			fields.ECCM31.FieldModulus(),
			&circuit,
		)
		require.NoError(t, err, "ggs compile circuit error")
		layeredCircuit := circuitCompileResult.GetLayeredCircuit()

		inputSolver := circuitCompileResult.GetInputSolver()
		witness, err := inputSolver.SolveInput(&testcase.Assignment, 0)
		require.NoError(t, err, "ggs solving witness error")

		require.True(
			t,
			test.CheckCircuit(layeredCircuit, witness),
			"ggs check circuit error",
		)
	}
}
