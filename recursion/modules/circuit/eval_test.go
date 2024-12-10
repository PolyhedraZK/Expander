package circuit

import (
	"math/big"
	"testing"

	// "github.com/PolyhedraZK/ExpanderCompilerCollection/field/m31"
	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/stretchr/testify/require"
	"golang.org/x/exp/rand"
)

type Evaluation struct {
	Circuit      Circuit // public input is part of circuit, see the definition
	PrivateInput [][]frontend.Variable
}

func (e *Evaluation) Define(api frontend.API) error {
	api.Println("Definition start")
	n_witnesses := len(e.PrivateInput)
	for i := 0; i < n_witnesses; i++ {
		cur_input := e.PrivateInput[i]
		for j := 0; j < len(e.Circuit.Layers); j++ {
			layer := &e.Circuit.Layers[j]

			cur_output := make([]frontend.Variable, uint(1)<<layer.OutputLenLog)
			for k := 0; k < len(cur_output); k++ {
				cur_output[k] = 0
			}

			for k := 0; k < len(layer.Mul); k++ {
				mul_gate := layer.Mul[k]
				cur_output[mul_gate.OId] = api.Add(cur_output[mul_gate.OId],
					api.Mul(cur_input[mul_gate.IIds[0]], cur_input[mul_gate.IIds[1]], mul_gate.Coef.GetActualLocalValue()),
				)
			}

			for k := 0; k < len(layer.Add); k++ {
				add_gate := layer.Add[k]
				cur_output[add_gate.OId] = api.Add(cur_output[add_gate.OId],
					api.Mul(cur_input[add_gate.IIds[0]], add_gate.Coef.GetActualLocalValue()),
				)
			}

			for k := 0; k < len(layer.Cst); k++ {
				cst_gate := layer.Cst[k]
				var v frontend.Variable
				if cst_gate.Coef.CoefType == PublicInput {
					v = e.Circuit.PublicInput[i][cst_gate.Coef.InputIdx]
				} else if cst_gate.Coef.CoefType == Constant {
					v = cst_gate.Coef.Value
				} else {
					v = cst_gate.Coef.RandomValue
				}

				cur_output[cst_gate.OId] = api.Add(cur_output[cst_gate.OId], v)
			}

			cur_input = cur_output
		}

		api.Println("wit: #", i)
		for j := uint(0); j < e.Circuit.ExpectedNumOutputZeros; j++ {
			api.Println(cur_input[j])
			api.AssertIsEqual(cur_input[j], 0)
		}
	}

	return nil
}

type CircuitForTest struct {
	pathToCircuit string
	pathToWitness string
	fieldSimd     uint
	field         *big.Int
}

func (c CircuitForTest) load_circuit_and_witness() (*Circuit, [][]frontend.Variable) {
	// TODO field size? bytes?
	return ReadCircuit(c.pathToCircuit, c.pathToWitness, c.fieldSimd)
}

func TestCircuitEvaluation(t *testing.T) {
	testCircuitEvaluationHelper(t, CircuitForTest{
		pathToCircuit: "../../../data/circuit_bn254.txt",
		pathToWitness: "../../../data/witness_bn254.txt",
		fieldSimd:     1,
		field:         ecc.BN254.ScalarField(),
	})
	// testCircuitEvaluationHelper(t, CircuitForTest{
	// 	pathToCircuit: "../../../data/circuit_m31.txt",
	// 	pathToWitness: "../../../data/witness_m31.txt",
	// 	fieldSimd:     1,
	// 	field:         m31.ScalarField,
	// })
}

func testCircuitEvaluationHelper(t *testing.T, circuitForTest CircuitForTest) {
	circuit, private_input := circuitForTest.load_circuit_and_witness()

	println(circuit.ExpectedNumOutputZeros)
	for i := 0; i < len(circuit.PublicInput[0]); i++ {
		v, _ := circuit.PublicInput[0][i].(big.Int)
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
	r1cs, err := frontend.Compile(circuitForTest.field, r1cs.NewBuilder, &evaluation)
	require.NoError(t, err, "Unable to generate r1cs")

	println("Nb Constraints: ", r1cs.GetNbConstraints())
	println("Nb Internal Witnesss: ", r1cs.GetNbInternalVariables())
	println("Nb Private Witness: ", r1cs.GetNbSecretVariables())
	println("Nb Public Witness:", r1cs.GetNbPublicVariables())

	// Correct Witness
	circuit, private_input = circuitForTest.load_circuit_and_witness()

	assignment := Evaluation{
		Circuit:      *circuit,
		PrivateInput: private_input,
	}
	witness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	require.NoError(t, err, "Unable to solve witness")

	err = r1cs.IsSolved(witness)
	require.NoError(t, err, "R1CS not satisfied")

	// Incorrect witness
	circuit, private_input = circuitForTest.load_circuit_and_witness()
	ri := rand.Intn(len(private_input))
	rj := rand.Intn(len(private_input[0]))
	private_input[ri][rj] = 147258369 // this should make the evaluation incorrect

	assignment = Evaluation{
		Circuit:      *circuit,
		PrivateInput: private_input,
	}
	witness, err = frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	require.NoError(t, err, "Unable to solve witness")

	err = r1cs.IsSolved(witness)
	require.Error(t, err, "Incorrect witness should not be marked as solved")
}
