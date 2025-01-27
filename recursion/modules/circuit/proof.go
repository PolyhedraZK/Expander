package circuit

import (
	"ExpanderVerifierCircuit/modules/fields"

	"github.com/consensys/gnark/frontend"
)

type Proof struct {
	Idx   uint
	Elems []frontend.Variable
}

func (p *Proof) Next() frontend.Variable {
	var e = p.Elems[p.Idx]
	p.Idx++

	return e
}

func (p *Proof) NextChallengeF(api fields.ArithmeticEngine) []frontend.Variable {
	challengeDegree := api.ChallengeFieldDegree()
	temp := make([]frontend.Variable, challengeDegree)
	for i := uint(0); i < challengeDegree; i++ {
		temp[i] = p.Next()
	}

	return temp
}

func (p *Proof) Reset() {
	p.Idx = 0
}

func (p *Proof) PlaceHolder() *Proof {
	return &Proof{
		Idx:   0,
		Elems: make([]frontend.Variable, len(p.Elems)),
	}
}

func NewRandomProof(n_elems uint) *Proof {
	var proof = Proof{}

	proof.Idx = 0
	for i := uint(0); i < n_elems; i++ {
		proof.Elems = append(proof.Elems, uint(123456789))
	}

	return &proof
}
