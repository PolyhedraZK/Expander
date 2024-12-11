package circuit

import (
	"math/big"
	"testing"

	"github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo"
	ecgoTest "github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo/test"
	"github.com/consensys/gnark/frontend"
	"github.com/stretchr/testify/require"
)

func TestCircuitLayeredEvaluation(t *testing.T) {
	testCircuitLayeredEvaluationHelper(t, CircuitRelation{
		pathToCircuit: "../../../data/circuit_bn254.txt",
		pathToWitness: "../../../data/witness_bn254.txt",
		mpiSize:       1,
		fieldEnum:     ECCBN254,
	})
	// testCircuitLayeredEvaluationHelper(t, CircuitRelation{
	// 	pathToCircuit: "../../../data/circuit_m31.txt",
	// 	pathToWitness: "../../../data/witness_m31.txt",
	// 	mpiSize:       1,
	// 	fieldEnum:     ECCM31,
	// })
	testCircuitLayeredEvaluationHelper(t, CircuitRelation{
		pathToCircuit: "../../../data/circuit_gf2.txt",
		pathToWitness: "../../../data/witness_gf2.txt",
		mpiSize:       1,
		fieldEnum:     ECCGF2,
	})
}

func testCircuitLayeredEvaluationHelper(t *testing.T, circuitRel CircuitRelation) {
	circuit, privateInput, err := ReadCircuit(circuitRel)
	require.NoError(t, err)

	println(circuit.ExpectedNumOutputZeros)
	for i := 0; i < len(circuit.PublicInput[0]); i++ {
		v, cast := circuit.PublicInput[0][i].(big.Int)
		require.True(t, cast)
		println("Public Input", v.String())
	}

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
