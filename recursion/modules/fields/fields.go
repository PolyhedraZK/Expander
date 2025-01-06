package fields

import (
	"math/big"

	eccFields "github.com/PolyhedraZK/ExpanderCompilerCollection/ecgo/field"
	"github.com/consensys/gnark/frontend"
)

// ECCFieldEnum is the enum value indicating the field that GKR proof relies on
type ECCFieldEnum uint64

// The enum assignment is aligning with the ones on ECGO side.
const (
	// ECCBN254 is the ECCFieldEnum for BN254 field
	ECCBN254 ECCFieldEnum = 2
	// ECCM31 is the ECCFieldEnum for Mersenne31 field
	ECCM31 ECCFieldEnum = 1
	// ECCGF2 is the ECCFieldEnum for Galois2 field
	ECCGF2 ECCFieldEnum = 3
)

func (f ECCFieldEnum) GetFieldEngine() eccFields.Field {
	return eccFields.GetFieldById(uint64(f))
}

// FieldModulus finds the modulus for the base field tied to the ECC field enum
func (f ECCFieldEnum) FieldModulus() *big.Int {
	fieldEngine := f.GetFieldEngine()
	return fieldEngine.Field()
}

// FieldBytes stand for the number of bytes of the base field modulus
// tied to the ECC field enum
func (f ECCFieldEnum) FieldBytes() uint {
	fieldModulus := f.FieldModulus()
	bitLen := fieldModulus.BitLen()
	// NOTE: round up against bit-byte rate
	return (uint(bitLen) + 8 - 1) / 8
}

// SIMDPackSize stands for the SIMD input packing size tied to the circuit field
func (f ECCFieldEnum) SIMDPackSize() uint {
	switch f {
	case ECCBN254:
		return 1
	case ECCM31:
		return 16
	case ECCGF2:
		return 8
	default:
		panic("bruh wyd here, you aint ecc field enum yo?")
	}
}

// ChallengeFieldDegree is the degree of the challenge field, that is the
// polynomial extension field of the circuit field (base field)
func (f ECCFieldEnum) ChallengeFieldDegree() uint {
	switch f {
	case ECCBN254:
		return 1
	case ECCM31:
		return 3
	case ECCGF2:
		return 128
	default:
		panic("bruh we are talking bout challenge field degree, whotf are you?")
	}
}

// ArithmeticEngine extends from frontend.API to handle extension field
// Addition/Substraction/Multiplication.
type ArithmeticEngine struct {
	ECCFieldEnum
	frontend.API
}

// AssertEq checks if a bunch of base field elements equal to each other
// assuming they are limbs of an extension field element.
func (engine *ArithmeticEngine) AssertEq(
	lhs []frontend.Variable, rhs []frontend.Variable) {

	degree := engine.ChallengeFieldDegree()
	if len(lhs) != int(degree) || len(rhs) != int(degree) {
		panic("extension field should be of same degree")
	}

	for i := range lhs {
		engine.API.AssertIsEqual(lhs[i], rhs[i])
	}
}

// Zero returns extension field zero instance.
func (engine *ArithmeticEngine) Zero() []frontend.Variable {
	degree := engine.ChallengeFieldDegree()
	zero := make([]frontend.Variable, degree)
	for i := 0; i < int(degree); i++ {
		zero[i] = 0
	}
	return zero
}

// Zeroes returns a slice of extension field zero instances.
func (engine *ArithmeticEngine) Zeroes(num uint) [][]frontend.Variable {
	res := make([][]frontend.Variable, num)
	for i := uint(0); i < num; i++ {
		res[i] = engine.Zero()
	}

	return res
}

// One returns extension field one instance.
func (engine *ArithmeticEngine) One() []frontend.Variable {
	one := engine.Zero()
	one[0] = 1
	return one
}

// ToExtension lifts a base field element to an extension field element.
func (engine *ArithmeticEngine) ToExtension(
	e frontend.Variable) []frontend.Variable {

	res := engine.Zero()
	res[0] = e
	return res
}

// ExtensionAdd adds a bunch of base field elements to each other
// assuming they are limbs of an extension field element.
func (engine *ArithmeticEngine) ExtensionAdd(
	e0 []frontend.Variable,
	e1 []frontend.Variable,
	es ...[]frontend.Variable) []frontend.Variable {

	degree := engine.ChallengeFieldDegree()
	if len(e0) != int(degree) || len(e1) != int(degree) {
		panic("extension field should be of same degree")
	}

	for _, e := range es {
		if len(e) != int(degree) {
			panic("extension field should be of same degree")
		}
	}

	res := make([]frontend.Variable, degree)
	es_at_i := make([]frontend.Variable, len(es))
	for i := 0; i < int(degree); i++ {
		for j := 0; j < len(es); j++ {
			es_at_i[j] = es[j][i]
		}

		res[i] = engine.Add(e0[i], e1[i], es_at_i...)
	}

	return res
}

// ExtensionSub substracts a bunch of base field elements to each other
// assuming they are limbs of an extension field element.
func (engine *ArithmeticEngine) ExtensionSub(
	e0 []frontend.Variable,
	e1 []frontend.Variable,
	es ...[]frontend.Variable) []frontend.Variable {

	degree := engine.ChallengeFieldDegree()
	if len(e0) != int(degree) || len(e1) != int(degree) {
		panic("extension field should be of same degree")
	}

	for _, e := range es {
		if len(e) != int(degree) {
			panic("extension field should be of same degree")
		}
	}

	res := make([]frontend.Variable, degree)
	es_at_i := make([]frontend.Variable, len(es))
	for i := 0; i < int(degree); i++ {
		for j := 0; j < len(es); j++ {
			es_at_i[j] = es[j][i]
		}

		res[i] = engine.Sub(e0[i], e1[i], es_at_i...)
	}

	return res
}

// ExtensionMul multiplies a bunch of base field elements to each other
// assuming they are limbs of an extension field element.
func (engine *ArithmeticEngine) ExtensionMul(
	e0 []frontend.Variable,
	e1 []frontend.Variable,
	es ...[]frontend.Variable) []frontend.Variable {

	degree := engine.ChallengeFieldDegree()
	if len(e0) != int(degree) || len(e1) != int(degree) {
		panic("extension field should be of same degree")
	}

	for _, e := range es {
		if len(e) != int(degree) {
			panic("extension field should be of same degree")
		}
	}

	res := engine.pairwiseExtensionMul(e0, e1)
	for _, e := range es {
		res = engine.pairwiseExtensionMul(res, e)
	}

	return res
}

// pairwiseExtensionMul multiplies a pair of base field element slices
// assuming they are limbs of an extension field element.
func (engine *ArithmeticEngine) pairwiseExtensionMul(
	e0 []frontend.Variable, e1 []frontend.Variable) []frontend.Variable {

	res := engine.Zero()

	switch engine.ECCFieldEnum {
	case ECCBN254:
		res[0] = engine.Mul(e0[0], e1[0])
	case ECCM31:
		// polynomial mod (x^3 - 5)
		//
		//   (a0 + a1*x + a2*x^2) * (b0 + b1*x + b2*x^2) mod (x^3 - 5)
		// = a0*b0 + (a0*b1 + a1*b0)*x + (a0*b2 + a1*b1 + a2*b0)*x^2
		// + (a1*b2 + a2*b1)*x^3 + a2*b2*x^4 mod (x^3 - 5)
		// = a0*b0 + 5*(a1*b2 + a2*b1)
		// + ((a0*b1 + a1*b0) + 5*a2*b2)*x
		// + (a0*b2 + a1*b1 + a2*b0)*x^2

		res[0] = engine.Add(
			engine.Mul(e0[0], e1[0]),
			engine.Mul(
				engine.Add(
					engine.Mul(e0[1], e1[2]),
					engine.Mul(e0[2], e1[1]),
				),
				5,
			),
		)

		res[1] = engine.Add(
			engine.Mul(e0[0], e1[1]),
			engine.Mul(e0[1], e1[0]),
			engine.Mul(e0[2], e1[2], 5),
		)

		res[2] = engine.Add(
			engine.Mul(e0[0], e1[2]),
			engine.Mul(e0[1], e1[1]),
			engine.Mul(e0[2], e1[0]),
		)
	case ECCGF2:
		// TODO(HS) implement GF2_128 pairwise multiplication
		fallthrough
	default:
		panic("extension field multiplication not yet supported")
	}

	return res
}
