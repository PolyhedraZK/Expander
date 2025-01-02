package main

import (
	"os"

	"ExpanderVerifierCircuit/modules/circuit"
	"ExpanderVerifierCircuit/modules/fields"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/spf13/cobra"
)

var (
	groth16CRSFile string
	groth16VKFile  string
	groth16Mode    string
)

var groth16Cmd = &cobra.Command{
	Use:   "groth16",
	Short: "Convert a single GKR proof into a Groth16 proof",
	Args:  cobra.NoArgs,
	Run: func(cmd *cobra.Command, args []string) {
		Groth16RecursionImpl()
	},
}

func init() {
	recursionCmd.AddCommand(groth16Cmd)
	groth16Cmd.PersistentFlags().StringVar(&groth16CRSFile, "groth16-crs", "", "The Groth16 CRS used in Groth16 recursion proof.")
	groth16Cmd.PersistentFlags().StringVar(&groth16VKFile, "groth16-vk", "", "The Groth16 VK used in Groth16 recursion proof.")
	groth16Cmd.PersistentFlags().StringVar(&groth16Mode, "groth16-mode", "", "The Groth16 recursion work mode - one of prove/verify/setup.")
}

func Groth16RecursionImpl() {
	groth16CircuitRel := circuit.CircuitRelation{
		CircuitPath: circuitFile,
		WitnessPath: witnessFiles[0],
		FieldEnum:   fields.ECCBN254,
		MPISize:     mpiSize,
	}
	originalCircuit, _, err := circuit.ReadCircuit(groth16CircuitRel)
	if err != nil {
		panic(err.Error())
	}

	proof, err := circuit.ReadProofFile(gkrProofFiles[0], fields.ECCBN254)
	if err != nil {
		panic(err.Error())
	}

	originalCircuit.PrintStats()

	verifier_circuit := VerifierCircuit{
		MpiSize:         mpiSize,
		FieldEnum:       fields.ECCBN254,
		OriginalCircuit: *originalCircuit,
		Proof:           *proof.PlaceHolder(),
	}
	r1cs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &verifier_circuit)
	if err != nil {
		panic(err.Error())
	}

	println("Nb Constraints: ", r1cs.GetNbConstraints())
	println("Nb Internal Witnesss: ", r1cs.GetNbInternalVariables())
	println("Nb Private Witness: ", r1cs.GetNbSecretVariables())
	println("Nb Public Witness:", r1cs.GetNbPublicVariables())

	// witness definition
	originalCircuit, _, err = circuit.ReadCircuit(groth16CircuitRel)
	if err != nil {
		panic(err.Error())
	}

	assignment := VerifierCircuit{
		MpiSize:         mpiSize,
		FieldEnum:       fields.ECCBN254,
		OriginalCircuit: *originalCircuit,
		Proof:           *proof,
	}

	println("Solving witness...")
	witness, err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	if err != nil {
		panic(err.Error())
	}

	println("Checking satisfiability...")
	if err = r1cs.IsSolved(witness); err != nil {
		panic("R1CS not satisfied.")
	}
	println("R1CS satisfied.")

	pk := groth16.NewProvingKey(ecc.BN254)
	vk := groth16.NewVerifyingKey(ecc.BN254)
	groth16Proof := groth16.NewProof(ecc.BN254)

	var pkFile *os.File = nil
	var vkFile *os.File = nil
	var proofFile *os.File = nil

	switch groth16Mode {
	case "setup":
		println("Groth16 generating setup from scratch...")
		if pk, vk, err = groth16.Setup(r1cs); err != nil {
			panic(err.Error())
		}

		if pkFile, err = os.OpenFile(groth16CRSFile,
			os.O_WRONLY|os.O_CREATE, 0644); err != nil {
			panic(err.Error())
		}
		pk.WriteTo(pkFile)

		if vkFile, err = os.OpenFile(groth16VKFile,
			os.O_WRONLY|os.O_CREATE, 0644); err != nil {
			panic(err.Error())
		}
		vk.WriteTo(vkFile)
	case "prove":
		println("Groth16 reading CRS from file...")
		if pkFile, err = os.OpenFile(groth16CRSFile, os.O_RDONLY, 0444); err != nil {
			panic(err.Error())
		}
		pk.ReadFrom(pkFile)

		groth16Proof, err = groth16.Prove(r1cs, pk, witness)
		if err != nil {
			panic("Groth16 fails")
		}

		if proofFile, err = os.OpenFile(recursiveProofFile,
			os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0644); err != nil {
			panic(err.Error())
		}
		groth16Proof.WriteTo(proofFile)
	case "verify":
		println("Groth16 reading vk from file...")
		if vkFile, err = os.OpenFile(groth16VKFile, os.O_RDONLY, 0444); err != nil {
			panic(err.Error())
		}
		vk.ReadFrom(vkFile)

		if proofFile, err = os.OpenFile(recursiveProofFile, os.O_RDONLY, 0444); err != nil {
			panic(err.Error())
		}
		groth16Proof.ReadFrom(proofFile)

		publicWitness, err := witness.Public()
		if err != nil {
			panic(err.Error())
		}

		if err = groth16.Verify(groth16Proof, vk, publicWitness); err != nil {
			panic(err.Error())
		}
	}

	println("Done.")
}
