package circuit

import (
	"math/big"
	"testing"

	"github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo"
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
	// 	mpiSize:       8,
	// 	fieldEnum:     ECCM31,
	// })
	// testCircuitLayeredEvaluationHelper(t, CircuitRelation{
	// 	pathToCircuit: "../../../data/circuit_gf2.txt",
	// 	pathToWitness: "../../../data/witness_gf2.txt",
	// 	mpiSize:       8,
	// 	fieldEnum:     ECCGF2,
	// })
}

func testCircuitLayeredEvaluationHelper(t *testing.T, circuitRel CircuitRelation) {
	circuit, private_input, err := ReadCircuit(circuitRel)
	require.NoError(t, err)
	fieldModulus, err := circuitRel.fieldEnum.FieldModulus()
	require.NoError(t, err)

	println(circuit.ExpectedNumOutputZeros)
	for i := 0; i < len(circuit.PublicInput[0]); i++ {
		v, cast := circuit.PublicInput[0][i].(big.Int)
		require.True(t, cast)
		println("Public Input", v.String())
	}

	public_input_empty := make([][]frontend.Variable, len(circuit.PublicInput))
	for i := 0; i < len(public_input_empty); i++ {
		public_input_empty[i] = make([]frontend.Variable, len(circuit.PublicInput[0]))
	}
	circuit.PublicInput = public_input_empty

	private_input_empty := make([][]frontend.Variable, len(private_input))
	for i := 0; i < len(private_input_empty); i++ {
		private_input_empty[i] = make([]frontend.Variable, len(private_input[0]))
	}

	evaluation := Evaluation{
		Circuit:      *circuit,
		PrivateInput: private_input_empty,
	}

	_, err = ecgo.Compile(fieldModulus, &evaluation)
	require.NoError(t, err, "ECGO compilation error")
}
