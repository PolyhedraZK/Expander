package polycommit

import "github.com/consensys/gnark/frontend"

type RawCommitment struct {
	Vals []frontend.Variable
}

func EvalMultilinear(api frontend.API, vs []frontend.Variable, r []frontend.Variable) frontend.Variable {
	if 1<<len(r) != len(vs) {
		panic("Inconsistent length of vals and randomness in eval multi-linear")
	}

	scratch := make([]frontend.Variable, len(vs))
	copy(scratch, vs)

	cur_eval_size := len(vs) >> 1
	for i := 0; i < len(r); i++ {
		for j := 0; j < cur_eval_size; j++ {
			scratch[j] = api.Add(scratch[2*j], api.Mul(
				api.Sub(scratch[2*j+1], scratch[2*j]),
				r[i],
			))
		}
	}
	return scratch[0]
}

func (c *RawCommitment) Verify(api frontend.API, r []frontend.Variable, y frontend.Variable) {
	api.AssertIsEqual(EvalMultilinear(api, c.Vals, r), y)
}

func NewRawCommitment(vals []frontend.Variable) *RawCommitment {
	return &RawCommitment{
		Vals: vals,
	}
}
