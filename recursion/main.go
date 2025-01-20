package main

import (
	"fmt"
	"os"

	"ExpanderVerifierCircuit/modules/circuit"
	"ExpanderVerifierCircuit/modules/fields"
	"ExpanderVerifierCircuit/modules/verifier"

	"github.com/consensys/gnark/frontend"
	"github.com/spf13/cobra"
)

type VerifierCircuit struct {
	MpiSize         uint
	FieldEnum       fields.ECCFieldEnum
	OriginalCircuit circuit.Circuit
	Proof           circuit.Proof // private input
}

// Define declares the circuit constraints
func (circuit *VerifierCircuit) Define(api frontend.API) error {
	arithmeticEngine := fields.ArithmeticEngine{ECCFieldEnum: circuit.FieldEnum, API: api}
	zero := arithmeticEngine.Zero()

	verifier.Verify(
		arithmeticEngine,
		circuit.FieldEnum,
		&circuit.OriginalCircuit,
		circuit.OriginalCircuit.PublicInput,
		zero,
		circuit.MpiSize,
		&circuit.Proof,
	)
	return nil
}

var (
	circuitFile   string
	witnessFiles  []string
	gkrProofFiles []string

	recursiveProofFile string

	mpiSize uint
)

func init() {
	recursionCmd.PersistentFlags().StringVar(&circuitFile, "circuit-file", "", "The circuit tied to the GKR proofs fed into the recursive circuit.")
	recursionCmd.PersistentFlags().StringSliceVar(&witnessFiles, "witness-files", nil, "The witness for GKR proofs fed into the recursion circuit.")
	recursionCmd.PersistentFlags().StringSliceVar(&gkrProofFiles, "gkr-proofs", nil, "The GKR proofs need to be fed into the recursion circuit.")
	recursionCmd.PersistentFlags().UintVar(&mpiSize, "mpi-size", 0, "The MPI size used to generate the GKR proofs.")
	recursionCmd.PersistentFlags().StringVar(&recursiveProofFile, "recursion-proof", "", "The recursion proof output file.")

	recursionCmd.MarkFlagRequired("circuit-file")
	recursionCmd.MarkFlagRequired("witness-files")
	recursionCmd.MarkFlagRequired("gkr-proofs")
	recursionCmd.MarkFlagRequired("mpi-size")
}

var recursionCmd = &cobra.Command{
	Use:   "recursion",
	Short: "Manage Recursion proof generation",
	Args:  cobra.NoArgs,
	Run: func(cmd *cobra.Command, args []string) {
		cmd.HelpFunc()(cmd, args)
	},
}

func main() {
	if err := recursionCmd.Execute(); err != nil {
		fmt.Println(err.Error())
		os.Exit(1)
	}
}
