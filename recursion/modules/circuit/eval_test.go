package circuit

import (
	"fmt"
	"math/big"
	"testing"

	"github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo"
	ecgoTest "github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo/test"
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
	// api.Println("Definition start")
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

		// api.Println("wit: #", i)
		for j := uint(0); j < e.Circuit.ExpectedNumOutputZeros; j++ {
			// api.Println(cur_input[j])
			api.AssertIsEqual(cur_input[j], 0)
		}
	}

	return nil
}

func TestCircuitGnarkEvaluation(t *testing.T) {
	testCircuitGnarkEvaluationHelper(t, CircuitRelation{
		pathToCircuit: "../../../data/circuit_bn254.txt",
		pathToWitness: "../../../data/witness_bn254.txt",
		mpiSize:       1,
		fieldEnum:     ECCBN254,
	})
}

func testCircuitGnarkEvaluationHelper(t *testing.T, circuitForTest CircuitRelation) {
	circuit, private_input, err := ReadCircuit(circuitForTest)
	require.NoError(t, err)
	fieldModulus, err := circuitForTest.fieldEnum.FieldModulus()
	require.NoError(t, err)

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

	r1cs, err := frontend.Compile(fieldModulus, r1cs.NewBuilder, &evaluation)
	require.NoError(t, err, "Unable to generate r1cs")

	println("Nb Constraints: ", r1cs.GetNbConstraints())
	println("Nb Internal Witnesss: ", r1cs.GetNbInternalVariables())
	println("Nb Private Witness: ", r1cs.GetNbSecretVariables())
	println("Nb Public Witness:", r1cs.GetNbPublicVariables())

	// Correct Witness
	circuit, private_input, err = ReadCircuit(circuitForTest)
	require.NoError(t, err)

	assignment := Evaluation{
		Circuit:      *circuit,
		PrivateInput: private_input,
	}
	witness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	require.NoError(t, err, "Unable to solve witness")

	err = r1cs.IsSolved(witness)
	require.NoError(t, err, "R1CS not satisfied")

	// Incorrect witness
	circuit, private_input, err = ReadCircuit(circuitForTest)
	require.NoError(t, err)

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
