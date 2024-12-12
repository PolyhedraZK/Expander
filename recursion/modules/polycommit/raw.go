package polycommit

import (
	"ExpanderVerifierCircuit/modules/circuit"
	"ExpanderVerifierCircuit/modules/transcript"

	"github.com/consensys/gnark/frontend"
)

type RawCommitment struct {
	Vals []frontend.Variable
}

func EvalMultilinear(
	api frontend.API,
	vs []frontend.Variable,
	r []frontend.Variable,
) frontend.Variable {
	if 1<<len(r) != len(vs) {
		panic("Inconsistent length of vals and randomness in eval multi-linear")
	}

	scratch := make([]frontend.Variable, len(vs))
	copy(scratch, vs)

	cur_eval_size := len(vs) >> 1
	for i := 0; i < len(r); i++ {
		for j := 0; j < cur_eval_size; j++ {
			scratch[j] = api.Add(
				scratch[2*j],
				api.Mul(api.Sub(scratch[2*j+1], scratch[2*j]), r[i]),
			)
		}
	}
	return scratch[0]
}

func (c *RawCommitment) Verify(
	api frontend.API,
	r []frontend.Variable,
	y frontend.Variable) {
	api.AssertIsEqual(EvalMultilinear(api, c.Vals, r), y)
}

func NewRawPolyCommitment(
	fieldEnum circuit.ECCFieldEnum,
	comLen uint,
	proof *circuit.Proof,
	transcript *transcript.Transcript,
) (rawComm *RawCommitment, err error) {
	fieldBytes, err := fieldEnum.FieldBytes()
	if err != nil {
		return
	}

	// NOTE(HS) maybe I should read the elements for raw comm length
	// and comapre with the circuit input size... but this one suffices for now
	rawComLengthElemNum := circuit.LEADING_FIELD_BYTES / fieldBytes
	rawComLengthElems := make([]frontend.Variable, rawComLengthElemNum)
	for i := 0; i < int(rawComLengthElemNum); i++ {
		rawComLengthElems[i] = proof.Next()
	}
	transcript.AppendFs(rawComLengthElems...)

	// TODO(HS) should we compare rawComLen against rawComLengthElemNum

	// raw commitment add to transcript
	vals := make([]frontend.Variable, comLen)
	for i := uint(0); i < comLen; i++ {
		vals[i] = proof.Next()
	}
	transcript.AppendFs(vals...)

	rawComm = &RawCommitment{Vals: vals}
	return
}
