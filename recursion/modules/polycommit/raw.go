package polycommit

import (
	"ExpanderVerifierCircuit/modules/circuit"
	"ExpanderVerifierCircuit/modules/fields"
	"ExpanderVerifierCircuit/modules/transcript"

	"github.com/consensys/gnark/frontend"
)

type RawCommitment struct {
	Vals []frontend.Variable
}

func EvalMultilinear(
	api fields.ArithmeticEngine,
	// NOTE(HS) base field evals
	vs []frontend.Variable,
	// NOTE(HS) extension field eval point
	r [][]frontend.Variable,
) []frontend.Variable {
	if 1<<len(r) != len(vs) {
		panic("Inconsistent length of vals and randomness in eval multi-linear")
	}

	buf := make([][]frontend.Variable, len(vs))
	for i, v := range vs {
		buf[i] = api.ToExtension(v)
	}

	for i := 0; i < len(r); i++ {
		halfHypercubeSize := len(vs) >> (i + 1)
		for j := 0; j < halfHypercubeSize; j++ {
			buf[j] = api.ExtensionAdd(
				buf[2*j],
				api.ExtensionMul(api.ExtensionSub(buf[2*j+1], buf[2*j]), r[i]),
			)
		}
	}
	return buf[0]
}

func (c *RawCommitment) Verify(
	api fields.ArithmeticEngine,
	rs, rSIMD, rMPI [][]frontend.Variable,
	y []frontend.Variable,
) {
	totalNumVars := len(rs) + len(rSIMD) + len(rMPI)

	if 1<<len(rSIMD) != api.SIMDPackSize() {
		panic("Inconsistent SIMD length with randomness")
	}

	challengePoint := make([][]frontend.Variable, totalNumVars)
	copy(challengePoint, rSIMD)
	copy(challengePoint[len(rSIMD):], rs)
	copy(challengePoint[len(rSIMD)+len(rs):], rMPI)

	api.AssertEq(EvalMultilinear(api, c.Vals, challengePoint), y)
}

func NewRawPolyCommitment(
	fieldEnum fields.ECCFieldEnum,
	comLen uint,
	proof *circuit.Proof,
	fsTranscript *transcript.FieldHasherTranscript,
) *RawCommitment {
	// NOTE(HS) maybe I should read the elements for raw comm length
	// and comapre with the circuit input size... but this one suffices for now
	rawComLengthElemNum := circuit.RAW_COMMITMENT_LENGTH_BYTES / fieldEnum.FieldBytes()
	rawComLengthElems := make([]frontend.Variable, rawComLengthElemNum)
	for i := 0; i < int(rawComLengthElemNum); i++ {
		rawComLengthElems[i] = proof.Next()
	}
	fsTranscript.AppendFs(rawComLengthElems...)

	// TODO(HS) should we compare rawComLen against rawComLengthElemNum

	// raw commitment add to transcript
	vals := make([]frontend.Variable, comLen)
	for i := uint(0); i < comLen; i++ {
		vals[i] = proof.Next()
	}
	fsTranscript.AppendFs(vals...)

	return &RawCommitment{Vals: vals}
}
