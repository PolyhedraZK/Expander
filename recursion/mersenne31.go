package main

import (
	"ExpanderVerifierCircuit/modules/circuit"
	"ExpanderVerifierCircuit/modules/fields"

	"github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo"
	ecgoTest "github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo/test"
	"github.com/spf13/cobra"
)

func init() {
	recursionCmd.AddCommand(m31Cmd)
}

var m31Cmd = &cobra.Command{
	Use:   "mersenne31",
	Short: "Generate a Mersenne31 GKR recursion proof for a Mersenne31 GKR proof",
	Args:  cobra.NoArgs,
	Run: func(cmd *cobra.Command, args []string) {
		Mersenne31RecursionImpl()
	},
}

func Mersenne31RecursionImpl() {

	circuitRel := circuit.CircuitRelation{
		CircuitPath: circuitFile,
		WitnessPath: witnessFiles[0],
		FieldEnum:   fields.ECCM31,
		MPISize:     mpiSize,
	}
	originalCircuit, _, err := circuit.ReadCircuit(circuitRel)
	if err != nil {
		panic(err.Error())
	}

	// TODO(HS) read maybe more than just a single proof file, but rather multiple GKR proofs?
	proof, err := circuit.ReadProofFile(gkrProofFiles[0], fields.ECCM31)
	if err != nil {
		panic(err.Error())
	}

	originalCircuit.PrintStats()

	m31RecursionCircuit := VerifierCircuit{
		MpiSize:         mpiSize,
		FieldEnum:       fields.ECCM31,
		OriginalCircuit: *originalCircuit,
		Proof:           *proof.PlaceHolder(),
	}
	m31Compilation, err := ecgo.Compile(fields.ECCM31.FieldModulus(), &m31RecursionCircuit)
	if err != nil {
		panic(err.Error())
	}

	// witness definition
	originalCircuit, _, err = circuit.ReadCircuit(circuitRel)
	if err != nil {
		panic(err.Error())
	}

	assignment := VerifierCircuit{
		MpiSize:         mpiSize,
		FieldEnum:       fields.ECCM31,
		OriginalCircuit: *originalCircuit,
		Proof:           *proof,
	}

	println("Solving witness...")
	inputSolver := m31Compilation.GetInputSolver()
	witness, err := inputSolver.SolveInput(&assignment, 0)
	if err != nil {
		panic(err.Error())
	}

	println("Checking satisfiability...")
	layeredCircuit := m31Compilation.GetLayeredCircuit()
	println(ecgoTest.CheckCircuit(layeredCircuit, witness))
}
