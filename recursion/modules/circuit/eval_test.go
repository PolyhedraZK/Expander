package circuit

import (
	"ExpanderVerifierCircuit/modules/fields"
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
	// NOTE(HS) commenting out the api println as it is not supported in ecgo
	// - reactivate after we have such functionality
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

func readCircuitForCompile(t *testing.T, circuitRel CircuitRelation) Evaluation {
	circuit, privateInput, err := ReadCircuit(circuitRel)
	require.NoError(t, err)

	emptyPubInput := make([][]frontend.Variable, len(circuit.PublicInput))
	for i := 0; i < len(emptyPubInput); i++ {
		emptyPubInput[i] = make([]frontend.Variable, len(circuit.PublicInput[0]))
	}
	circuit.PublicInput = emptyPubInput

	emptyPrivateInput := make([][]frontend.Variable, len(privateInput))
	for i := 0; i < len(emptyPrivateInput); i++ {
		emptyPrivateInput[i] = make([]frontend.Variable, len(privateInput[0]))
	}

	return Evaluation{
		Circuit:      *circuit,
		PrivateInput: emptyPrivateInput,
	}
}

func readCircuitForAssignment(t *testing.T, circuitRel CircuitRelation) Evaluation {
	circuit, privateInput, err := ReadCircuit(circuitRel)
	require.NoError(t, err)

	return Evaluation{
		Circuit:      *circuit,
		PrivateInput: privateInput,
	}
}

func TestCircuitGnarkEvaluation(t *testing.T) {
	testCircuitGnarkEvaluationHelper(t, CircuitRelation{
		CircuitPath: "../../../data/circuit_bn254.txt",
		WitnessPath: "../../../data/witness_bn254.txt",
		MPISize:     1,
		FieldEnum:   fields.ECCBN254,
	})
}

func testCircuitGnarkEvaluationHelper(t *testing.T, testcase CircuitRelation) {
	evaluation := readCircuitForCompile(t, testcase)

	fieldModulus := testcase.FieldEnum.FieldModulus()

	r1cs, err := frontend.Compile(fieldModulus, r1cs.NewBuilder, &evaluation)
	require.NoError(t, err, "Unable to generate r1cs")

	println("Nb Constraints: ", r1cs.GetNbConstraints())
	println("Nb Internal Witnesss: ", r1cs.GetNbInternalVariables())
	println("Nb Private Witness: ", r1cs.GetNbSecretVariables())
	println("Nb Public Witness:", r1cs.GetNbPublicVariables())

	// Correct Witness
	assignment := readCircuitForAssignment(t, testcase)
	witness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	require.NoError(t, err, "Unable to solve witness")

	println("Num of public input", assignment.Circuit.ExpectedNumOutputZeros)
	for i := 0; i < len(assignment.Circuit.PublicInput[0]); i++ {
		v, _ := assignment.Circuit.PublicInput[0][i].(big.Int)
		println("Public Input", v.String())
	}

	err = r1cs.IsSolved(witness)
	require.NoError(t, err, "R1CS not satisfied")

	// Incorrect witness
	circuit, privateInput, err := ReadCircuit(testcase)
	require.NoError(t, err)

	ri := rand.Intn(len(privateInput))
	rj := rand.Intn(len(privateInput[0]))
	privateInput[ri][rj] = 147258369 // this should make the evaluation incorrect

	assignment = Evaluation{
		Circuit:      *circuit,
		PrivateInput: privateInput,
	}
	witness, err = frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	require.NoError(t, err, "Unable to solve witness")

	err = r1cs.IsSolved(witness)
	require.Error(t, err, "Incorrect witness should not be marked as solved")
}

func TestCircuitLayeredEvaluation(t *testing.T) {
	testcases := []CircuitRelation{
		{
			CircuitPath: "../../../data/circuit_bn254.txt",
			WitnessPath: "../../../data/witness_bn254.txt",
			MPISize:     1,
			FieldEnum:   fields.ECCBN254,
		},
		// NOTE(HS) as of 2024/12/11, the compilation process of m31 circuit
		// takes more than 50GB of RAM, so run with cautious yall...
		// {
		// 	CircuitPath: "../../../data/circuit_m31.txt",
		// 	WitnessPath: "../../../data/witness_m31.txt",
		// 	MPISize:     1,
		// 	FieldEnum:   fields.ECCM31,
		// },
		{
			CircuitPath: "../../../data/circuit_gf2.txt",
			WitnessPath: "../../../data/witness_gf2.txt",
			MPISize:     1,
			FieldEnum:   fields.ECCGF2,
		},
	}

	for _, testcase := range testcases {
		t.Run(
			fmt.Sprintf(
				"Layered circuit load and test for %s",
				testcase.CircuitPath,
			),
			func(t *testing.T) {
				testCircuitLayeredEvaluationHelper(t, testcase)
			},
		)
	}
}

func testCircuitLayeredEvaluationHelper(t *testing.T, testcase CircuitRelation) {
	evaluation := readCircuitForCompile(t, testcase)

	fieldModulus := testcase.FieldEnum.FieldModulus()

	eccCompilationResult, err := ecgo.Compile(fieldModulus, &evaluation)
	require.NoError(t, err, "ECGO compilation error")

	layeredCircuit := eccCompilationResult.GetLayeredCircuit()
	inputSolver := eccCompilationResult.GetInputSolver()

	// NOTE: get correct witness
	assignment := readCircuitForAssignment(t, testcase)

	witness, err := inputSolver.SolveInputAuto(&assignment)
	require.NoError(t, err, "ECGO witness resolve error")

	require.True(t, ecgoTest.CheckCircuit(layeredCircuit, witness))
}
