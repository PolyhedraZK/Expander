package circuit

import (
	"log"
	"math/big"

	"ExpanderVerifierCircuit/modules/transcript"

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

func (l *Layer) FillRndCoef(fsTranscript *transcript.FieldHasherTranscript) {
	for i := 0; i < len(l.Mul); i++ {
		if l.Mul[i].Coef.CoefType == Random {
			l.Mul[i].Coef.RandomValue = fsTranscript.CircuitF()
		}
	}

	for i := 0; i < len(l.Add); i++ {
		if l.Add[i].Coef.CoefType == Random {
			l.Add[i].Coef.RandomValue = fsTranscript.CircuitF()
		}
	}

	for i := 0; i < len(l.Cst); i++ {
		if l.Cst[i].Coef.CoefType == Random {
			l.Cst[i].Coef.RandomValue = fsTranscript.CircuitF()
		}
	}
}

func (c *Circuit) FillRndCoef(fsTranscript *transcript.FieldHasherTranscript) {
	for i := 0; i < len(c.Layers); i++ {
		c.Layers[i].FillRndCoef(fsTranscript)
	}
}

func (c *Circuit) PrintStats() {
	n_mul := 0
	n_add := 0
	n_cst_circuit := 0
	n_cst_input := 0

	for i := 0; i < len(c.Layers); i++ {
		n_mul += len(c.Layers[i].Mul)
		n_add += len(c.Layers[i].Add)
		n_cst_circuit += len(c.Layers[i].Cst)
	}

	n_cst_input = len(c.PublicInput[0])
	n_cst_circuit -= n_cst_input

	log.Println("#Layers: ", len(c.Layers))
	log.Println("#Mul Gates: ", n_mul)
	log.Println("#Add Gates: ", n_add)
	log.Println("#Cst Circuit: ", n_cst_circuit)
	log.Println("#Cst Input: ", n_cst_input)
}
