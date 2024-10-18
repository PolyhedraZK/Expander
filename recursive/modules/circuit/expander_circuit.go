package circuit

import (
	"ExpanderVerifierCircuit/modules/transcript"
	"math/big"

	"github.com/consensys/gnark/frontend"
)

type CoefType uint

const (
	Constant = iota
	Random
	PublicInput
)

type Coef struct {
	CoefType    CoefType
	Value       big.Int           // CoefType == Constant
	RandomValue frontend.Variable // CoefType == Random
	InputIdx    uint              // CoefType == PublicInput
}

func (c *Coef) GetActualLocalValue() frontend.Variable {
	switch c.CoefType {
	case Constant:
		return c.Value
	case Random:
		return c.RandomValue
	default:
		panic("Do not use this function for public input")
	}
}

type Gate struct {
	IIds []uint
	OId  uint

	Coef Coef
}

type StructureInfo struct {
	MaxDegreeOne bool
}

type Layer struct {
	InputLenLog  uint
	OutputLenLog uint

	Cst []Gate
	Add []Gate
	Mul []Gate

	StructureInfo StructureInfo
}

type Circuit struct {
	Layers      []Layer
	PublicInput [][]frontend.Variable `gnark:",public"`

	ExpectedNumOutputZeros uint
}

func (l *Layer) FillRndCoef(transcript *transcript.Transcript) {
	for i := 0; i < len(l.Mul); i++ {
		if l.Mul[i].Coef.CoefType == Random {
			l.Mul[i].Coef.RandomValue = transcript.ChallengeF()
		}
	}

	for i := 0; i < len(l.Add); i++ {
		if l.Add[i].Coef.CoefType == Random {
			l.Add[i].Coef.RandomValue = transcript.ChallengeF()
		}
	}

	for i := 0; i < len(l.Cst); i++ {
		if l.Cst[i].Coef.CoefType == Random {
			l.Cst[i].Coef.RandomValue = transcript.ChallengeF()
		}
	}
}

func (c *Circuit) FillRndCoef(transcript *transcript.Transcript) {
	for i := 0; i < len(c.Layers); i++ {
		c.Layers[i].FillRndCoef(transcript)
	}
}
