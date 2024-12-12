package polycommit

import (
	"ExpanderVerifierCircuit/modules/circuit"
	"ExpanderVerifierCircuit/modules/transcript"
	"fmt"

	"github.com/consensys/gnark/frontend"
)

// PolynomialCommitmentEnum is an enum value indicating which polynomial scheme
// that the GKR proof is using.
type PolynomialCommitmentEnum uint

const (
	// RawCommitmentScheme stands for using raw polynomial commitment scheme.
	RawCommitmentScheme PolynomialCommitmentEnum = iota
)

// PolynomialCommitment interface for GKR recursive verifier,
// only Verify method matters to me
type PolynomialCommitment interface {
	Verify(api frontend.API, r []frontend.Variable, y frontend.Variable)
}

func NewCommitment(
	schemeEnum PolynomialCommitmentEnum,
	fieldEnum circuit.ECCFieldEnum,
	circuitInputSize, mpiSize uint,
	proof *circuit.Proof,
	transcript *transcript.Transcript,
) (comm PolynomialCommitment, err error) {
	switch schemeEnum {
	case RawCommitmentScheme:
		comLen := circuitInputSize * mpiSize
		comm, err = NewRawPolyCommitment(fieldEnum, comLen, proof, transcript)
	default:
		err = fmt.Errorf("Unknown polynomial commitment scheme %d", schemeEnum)
	}
	return
}
