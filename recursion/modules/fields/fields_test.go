package fields

import (
	"testing"

	"github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo"
	"github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo/test"
	"github.com/consensys/gnark/frontend"
	"github.com/stretchr/testify/require"
)

type ExtensionFieldTestingCircuit struct {
	FieldEnum ECCFieldEnum

	LHS []frontend.Variable
	RHS []frontend.Variable

	Expected []frontend.Variable
}

func NewTestingCircuit(fieldEnum ECCFieldEnum) ExtensionFieldTestingCircuit {
	return ExtensionFieldTestingCircuit{
		FieldEnum: fieldEnum,
		LHS:       make([]frontend.Variable, fieldEnum.ChallengeFieldDegree()),
		RHS:       make([]frontend.Variable, fieldEnum.ChallengeFieldDegree()),
		Expected:  make([]frontend.Variable, fieldEnum.ChallengeFieldDegree()),
	}
}

func (c *ExtensionFieldTestingCircuit) Define(api frontend.API) error {
	arithmeticEngine := ArithmeticEngine{API: api, ECCFieldEnum: c.FieldEnum}
	actual := arithmeticEngine.ExtensionMul(c.LHS, c.RHS)
	arithmeticEngine.AssertEq(actual, c.Expected)

	return nil
}

func TestM31Ext3Arithmetic(t *testing.T) {
	circuit := NewTestingCircuit(ECCM31)
	circuitCompileResult, err := ecgo.Compile(
		ECCM31.FieldModulus(),
		&circuit,
	)
	require.NoError(t, err, "ggs compile circuit error")
	layeredCircuit := circuitCompileResult.GetLayeredCircuit()

	m31Assignment := ExtensionFieldTestingCircuit{
		FieldEnum: ECCM31,
		LHS:       []frontend.Variable{1, 2, 3},
		RHS:       []frontend.Variable{4, 5, 6},
		Expected:  []frontend.Variable{139, 103, 28},
	}
	inputSolver := circuitCompileResult.GetInputSolver()
	witness, err := inputSolver.SolveInput(&m31Assignment, 0)
	require.NoError(t, err, "ggs solving witness error")

	require.True(
		t,
		test.CheckCircuit(layeredCircuit, witness),
		"ggs check circuit error",
	)
}
