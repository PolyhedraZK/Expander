package main

import (
	"errors"
	"flag"
	"os"

	"ExpanderVerifierCircuit/modules/circuit"
	"ExpanderVerifierCircuit/modules/verifier"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/spf13/cobra"

	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
)

type VerifierCircuit struct {
	MpiSize         uint
	SimdSize        uint
	OriginalCircuit circuit.Circuit
	Proof           circuit.Proof // private input
}

// Define declares the circuit constraints
func (circuit *VerifierCircuit) Define(api frontend.API) error {
	verifier.Verify(api, &circuit.OriginalCircuit, circuit.OriginalCircuit.PublicInput, 0, circuit.SimdSize, circuit.MpiSize, &circuit.Proof)
	return nil
}

func checkFileExists(filePath string) bool {
	_, error := os.Stat(filePath)
	//return !os.IsNotExist(err)
	return !errors.Is(error, os.ErrNotExist)
}

// TODO cobra command line configuration
func init() {
	// TODO ...
}

var groth16Cmd = &cobra.Command{
	Use:   "groth16",
	Short: "Convert a single GKR proof into a Groth16 proof",
	Args:  cobra.NoArgs,
	Run: func(cmd *cobra.Command, args []string) {
		cmd.HelpFunc()(cmd, args)
	},
}

func testGroth16() {
	circuit_file := flag.String("circuit", "../data/circuit_bn254.txt", "circuit file")
	witness_file := flag.String("witness", "../data/witness_bn254.txt", "witness file")
	gkr_proof_file := flag.String("gkr_proof", "../data/gkr_proof.txt", "gkr proof file")

	with_groth16 := flag.Bool("with_groth16", false, "set true to do groth16 proof")
	groth16_pk_file := flag.String("groth16_pk", "", "where to put the proving key, will create a new one and write to this file if it does not exist")
	groth16_vk_file := flag.String("groth16_vk", "", "where to put the verifying key, will create a new one and write to this file if it does not exist")
	recursive_proof_file := flag.String("recursive_proof", "../data/recursive_proof.txt", "where to output the groth16 recursive proof")

	mpi_size := flag.Uint("mpi_size", 1, "mpi size of gkr proof")
	simd_size := flag.Uint("simd_size", 1, "simd size of gkr proof")
	flag.Parse()

	if *simd_size != 1 {
		panic("For bn254, Expander only implements simd size 1, so it must be 1 here")
	}

	// FIXME(HS): currently tied to only BN254
	groth16CircuitRel := circuit.CircuitRelation{
		CircuitPath: *circuit_file,
		WitnessPath: *witness_file,
		FieldEnum:   circuit.ECCBN254,
		MPISize:     *mpi_size,
	}
	original_circuit, _, err := circuit.ReadCircuit(groth16CircuitRel)
	if err != nil {
		panic(err.Error())
	}

	proof, err := circuit.ReadProofFile(*gkr_proof_file, circuit.ECCBN254)
	if err != nil {
		panic(err.Error())
	}

	original_circuit.PrintStats()

	verifier_circuit := VerifierCircuit{
		MpiSize:         *mpi_size,
		SimdSize:        *simd_size,
		OriginalCircuit: *original_circuit,
		Proof:           *proof.PlaceHolder(),
	}
	r1cs, _ := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &verifier_circuit)

	println("Nb Constraints: ", r1cs.GetNbConstraints())
	println("Nb Internal Witnesss: ", r1cs.GetNbInternalVariables())
	println("Nb Private Witness: ", r1cs.GetNbSecretVariables())
	println("Nb Public Witness:", r1cs.GetNbPublicVariables())

	// witness definition
	// FIXME(HS): currently tied to only BN254
	original_circuit, _, err = circuit.ReadCircuit(groth16CircuitRel)
	if err != nil {
		panic(err.Error())
	}

	assignment := VerifierCircuit{
		MpiSize:         *mpi_size,
		SimdSize:        *simd_size,
		OriginalCircuit: *original_circuit,
		Proof:           *proof,
	}

	println("Solving witness...")
	witness, witness_err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	if witness_err != nil {
		panic(witness_err.Error())
	}

	println("Checking satisfiability...")
	err = r1cs.IsSolved(witness)
	if err != nil {
		panic("R1CS not satisfied.")
	}
	println("R1CS satisfied.")

	if *with_groth16 {
		pk := groth16.NewProvingKey(ecc.BN254)
		vk := groth16.NewVerifyingKey(ecc.BN254)
		var setup_err error
		// groth16 zkSNARK: Setup
		if *groth16_pk_file != "" && *groth16_vk_file != "" &&
			checkFileExists(*groth16_pk_file) && checkFileExists(*groth16_vk_file) {
			println("Groth16 reading pk vk from file...", groth16_pk_file, " ", groth16_vk_file)
			pk_file, _ := os.OpenFile(*groth16_pk_file, os.O_RDONLY, 0444)
			pk.ReadFrom(pk_file)
			vk_file, _ := os.OpenFile(*groth16_vk_file, os.O_RDONLY, 0444)
			vk.ReadFrom(vk_file)
		} else {
			println("Groth16 generating setup from scratch...")
			pk, vk, setup_err = groth16.Setup(r1cs)

			pk_file, _ := os.OpenFile(*groth16_pk_file, os.O_WRONLY|os.O_CREATE, 0644)
			pk.WriteTo(pk_file)

			vk_file, _ := os.OpenFile(*groth16_vk_file, os.O_WRONLY|os.O_CREATE, 0644)
			vk.WriteTo(vk_file)
		}
		println("Setup done.")

		println("Groth16 prove-verify ing...")
		publicWitness, public_err := witness.Public()
		groth16_proof, prove_err := groth16.Prove(r1cs, pk, witness)
		verify_err := groth16.Verify(groth16_proof, vk, publicWitness)
		if setup_err != nil || public_err != nil || prove_err != nil || verify_err != nil {
			panic("Groth16 fails")
		}

		file, _ := os.OpenFile(*recursive_proof_file, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0644)
		groth16_proof.WriteTo(file)
	} else {
		println("Groth16 proof skipped, set '-with_groth16=1' to produce a proof")
	}

	println("Done.")
}

func main() {
	testGroth16()
}
