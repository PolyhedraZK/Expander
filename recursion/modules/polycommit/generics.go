package polycommit

import (
	"ExpanderVerifierCircuit/modules/circuit"
	"ExpanderVerifierCircuit/modules/fields"
	"ExpanderVerifierCircuit/modules/transcript"

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
	// Verify checks against commitment the opening point and eval
	// TODO(HS) for now this matches with raw commitment,
	// later we should add polynomial commitment opening to the interface
	Verify(
		api fields.ArithmeticEngine,
		rs, rSIMD, rMPI [][]frontend.Variable,
		y []frontend.Variable,
	)
}

// NewCommitment is the general interface for verifier circuit to extract a
// polynomial commitment out of the proof stream.  The side effect is adding the
// commitment frontend.Variables into the transcript, and polynomial commitment
// elements are read from proof data stream.
func NewCommitment(
	schemeEnum PolynomialCommitmentEnum,
	fieldEnum fields.ECCFieldEnum,
	circuitInputSize, mpiSize uint,
	proof *circuit.Proof,
	fsTranscript *transcript.FieldHasherTranscript,
) PolynomialCommitment {
	switch schemeEnum {
	case RawCommitmentScheme:
		comLen := circuitInputSize * mpiSize * fieldEnum.SIMDPackSize()
		return NewRawPolyCommitment(fieldEnum, comLen, proof, fsTranscript)
	default:
		panic("Unknown polynomial commitment scheme")
	}
}
