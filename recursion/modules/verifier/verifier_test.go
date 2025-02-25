package verifier

import (
	"ExpanderVerifierCircuit/modules/fields"
	"math/big"
	"os"
	"testing"

	"github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo"
	"github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo/irwg"
	"github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo/test"
	"github.com/consensys/gnark/frontend"
	"github.com/stretchr/testify/require"
)

type M31RecursionTestCircuit struct {
	A, B, X  frontend.Variable
	Expected frontend.Variable
}

// NOTE: circuit behavior: A + B \cdot X == Expected
func (c *M31RecursionTestCircuit) Define(api frontend.API) error {
	z := api.Add(c.A, api.Mul(c.B, c.X))
	api.AssertIsEqual(c.Expected, z)
	return nil
}

func TestM31CircuitForRecursionTestSatisfiability(t *testing.T) {
	circuitForRecursion, err := ecgo.Compile(fields.ECCM31.FieldModulus(), &M31RecursionTestCircuit{})
	require.NoError(t, err, "circuit compile error ggs")

	assignmentUnit := &M31RecursionTestCircuit{
		A:        10,
		B:        5,
		X:        2,
		Expected: 20,
	}

	inputSolver := circuitForRecursion.GetInputSolver()
	witnessUnit, err := inputSolver.SolveInput(assignmentUnit, 0)
	require.NoError(t, err, "solve witness error ggs")

	require.Equal(t, witnessUnit.NumWitnesses, 1, "witness unit should only have one witness")

	m31x16Values := make([]*big.Int, 16*len(witnessUnit.Values))
	for i := 0; i < 16; i++ {
		for j, value := range witnessUnit.Values {
			m31x16Values[i*len(witnessUnit.Values)+j] = value
		}
	}

	m31x16Witnesses := irwg.Witness{
		NumWitnesses:              16,
		NumInputsPerWitness:       witnessUnit.NumInputsPerWitness,
		NumPublicInputsPerWitness: witnessUnit.NumPublicInputsPerWitness,
		Field:                     witnessUnit.Field,
		Values:                    m31x16Values,
	}

	layeredCircuit := circuitForRecursion.GetLayeredCircuit()
	circuitChecks := test.CheckCircuitMultiWitness(layeredCircuit, &m31x16Witnesses)

	for _, check := range circuitChecks {
		require.True(t, check, "circuit witness check failed ggs")
	}

	os.WriteFile("../../../data/small_circuit_m31.txt", layeredCircuit.Serialize(), 0o644)
	os.WriteFile("../../../data/small_witness_m31.txt", m31x16Witnesses.Serialize(), 0o644)
}
