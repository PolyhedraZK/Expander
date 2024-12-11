package circuit

import (
	"fmt"
	"testing"

	"github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo"
	ecgoTest "github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo/test"
	"github.com/consensys/gnark/frontend"
	"github.com/stretchr/testify/require"
)

func TestCircuitLayeredEvaluation(t *testing.T) {
	testcases := []CircuitRelation{
		{
			pathToCircuit: "../../../data/circuit_bn254.txt",
			pathToWitness: "../../../data/witness_bn254.txt",
			mpiSize:       1,
			fieldEnum:     ECCBN254,
		},
		// NOTE(HS) as of 2024/12/11, the compilation process of m31 circuit
		// takes more than 50GB of RAM, so run with cautious
		{
			pathToCircuit: "../../../data/circuit_m31.txt",
			pathToWitness: "../../../data/witness_m31.txt",
			mpiSize:       1,
			fieldEnum:     ECCM31,
		},
		{
			pathToCircuit: "../../../data/circuit_gf2.txt",
			pathToWitness: "../../../data/witness_gf2.txt",
			mpiSize:       1,
			fieldEnum:     ECCGF2,
		},
	}

	for _, testcase := range testcases {
		t.Run(fmt.Sprintf("Layered circuit load and test for %s", testcase.pathToCircuit),
			func(t *testing.T) {
				testCircuitLayeredEvaluationHelper(t, testcase)
			},
		)
	}
}

func testCircuitLayeredEvaluationHelper(t *testing.T, circuitRel CircuitRelation) {
	circuit, privateInput, err := ReadCircuit(circuitRel)
	require.NoError(t, err)

	emptyPublicInput := make([][]frontend.Variable, len(circuit.PublicInput))
	for i := 0; i < len(emptyPublicInput); i++ {
		emptyPublicInput[i] = make([]frontend.Variable, len(circuit.PublicInput[0]))
	}
	circuit.PublicInput = emptyPublicInput

	emptyPrivateInput := make([][]frontend.Variable, len(privateInput))
	for i := 0; i < len(emptyPrivateInput); i++ {
		emptyPrivateInput[i] = make([]frontend.Variable, len(privateInput[0]))
	}

	evaluation := Evaluation{
		Circuit:      *circuit,
		PrivateInput: emptyPrivateInput,
	}

	fieldModulus, err := circuitRel.fieldEnum.FieldModulus()
	require.NoError(t, err)

	eccCompilationResult, err := ecgo.Compile(fieldModulus, &evaluation)
	require.NoError(t, err, "ECGO compilation error")

	layeredCircuit := eccCompilationResult.GetLayeredCircuit()
	inputSolver := eccCompilationResult.GetInputSolver()

	// NOTE: get correct witness
	circuit, privateInput, err = ReadCircuit(circuitRel)
	require.NoError(t, err)

	assignment := Evaluation{
		Circuit:      *circuit,
		PrivateInput: privateInput,
	}

	witness, err := inputSolver.SolveInputAuto(&assignment)
	require.NoError(t, err, "ECGO witness resolve error")

	require.True(t, ecgoTest.CheckCircuit(layeredCircuit, witness))
}
