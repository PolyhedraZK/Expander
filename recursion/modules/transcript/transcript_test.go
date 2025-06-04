package transcript

import (
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/stretchr/testify/require"

	"ExpanderVerifierCircuit/modules/fields"
)

type TranscriptTestingCircuit struct {
	Input  []frontend.Variable
	Output frontend.Variable
}

func (t *TranscriptTestingCircuit) Define(api frontend.API) error {
	arithmeticEngine := fields.ArithmeticEngine{API: api, ECCFieldEnum: fields.ECCBN254}
	transcript := NewTranscript(arithmeticEngine)
	transcript.AppendFs(t.Input...)
	computed_output := transcript.CircuitF()
	api.AssertIsEqual(computed_output, t.Output)
	return nil
}

// There is a corresponding rust script to produce the expected output
// located in transcript/src/tests.rs
func TestTranscript(t *testing.T) {
	circuit := TranscriptTestingCircuit{
		Input:  make([]frontend.Variable, 5),
		Output: frontend.Variable(0),
	}
	r1cs, r1cs_err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &circuit)

	require.NoError(t, r1cs_err, "ggs compile circuit error")

	assignment := TranscriptTestingCircuit{
		Input: []frontend.Variable{1, 2, 3, 4, 5},
		Output: func() frontend.Variable {
			v := new(big.Int)
			v.SetString("0x13f9a09b05c4429bbf9d0e782b00c942272a131a36749b2c55ba6ca3297ea9b7", 0)
			return v
		}(),
	}

	witness, witness_err := frontend.NewWitness(&assignment, ecc.BN254.ScalarField())
	require.NoError(t, witness_err, "ggs solving witness error")

	err := r1cs.IsSolved(witness)
	require.NoError(t, err, "ggs solving witness error")
}
