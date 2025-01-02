package verifier

import (
	"ExpanderVerifierCircuit/modules/fields"

	"github.com/consensys/gnark/frontend"
)

// eqEvalsAtPrimitive computes eq(r, x) for x \in {0, 1}^\ell,
// the primitive suffix means this method is using brute-force way.
func eqEvalsAtPrimitive(
	api fields.ArithmeticEngine,
	randomPoint [][]frontend.Variable,
	multiplicativeFactor []frontend.Variable,
	hypercubeEvals [][]frontend.Variable,
) {
	hypercubeEvals[0] = multiplicativeFactor

	for i, rI := range randomPoint {
		halfHypercubeSize := 1 << i

		for j := 0; j < halfHypercubeSize; j++ {
			// NOTE: apply new random variable to hypercube of evals
			// let previous eval being v, new variable being r,
			// then v -> ((1 - r) v, (r v)).
			hypercubeEvals[j+halfHypercubeSize] = api.ExtensionMul(
				hypercubeEvals[j],
				rI,
			)
			hypercubeEvals[j] = api.ExtensionSub(
				hypercubeEvals[j],
				hypercubeEvals[j+halfHypercubeSize],
			)
		}
	}
}

// EqEvalsAtEfficient computes eq(r, x) for x \in {0, 1}^\ell,
// but the efficiency lies in breaking down hypercubes into a tensor product
// of 2 sub-hypercubes:
// - one is formed by variables in randomPoint on the LHS,
// - while the other is constructed by variables in randomPoints on the RHS
func EqEvalsAtEfficient(
	api fields.ArithmeticEngine,
	randomPoint [][]frontend.Variable,
	multiplicativeFactor []frontend.Variable,
	fullHypercubeEvals [][]frontend.Variable,

	// NOTE: supplimentary helpful spaces for 1st and 2nd hypercube space.
	hypercubeEvals1stHalf, hypercubeEvals2ndHalf [][]frontend.Variable,

	eqEvalsCount map[uint]uint,
) {
	hypercubeSize := uint(1) << len(randomPoint)
	val, _ := eqEvalsCount[hypercubeSize]
	eqEvalsCount[hypercubeSize] = val + 1

	numVars1stHalf := uint(len(randomPoint) >> 1)

	eqEvalsAtPrimitive(
		api,
		randomPoint[:numVars1stHalf],
		multiplicativeFactor,
		hypercubeEvals1stHalf,
	)

	eqEvalsAtPrimitive(
		api,
		randomPoint[numVars1stHalf:],
		api.One(),
		hypercubeEvals2ndHalf,
	)

	firstHalfMask := (uint(1) << numVars1stHalf) - 1

	for i := uint(0); i < hypercubeSize; i++ {
		index1stHalf := i & firstHalfMask
		index2ndHalf := i >> numVars1stHalf
		fullHypercubeEvals[i] = api.ExtensionMul(
			hypercubeEvals1stHalf[index1stHalf],
			hypercubeEvals2ndHalf[index2ndHalf],
		)
	}
}

// CombineWithSimdMpi computes <values, mpiEvals \otimes simdEvals>.
func CombineWithSimdMpi(
	api fields.ArithmeticEngine,
	values, eqEvalsSIMDVars, eqEvalsMPIVars [][]frontend.Variable,
) []frontend.Variable {

	simdSize := int(api.SIMDPackSize())
	res := api.Zero()

	for i, mpiEval := range eqEvalsMPIVars {
		for j, simdEval := range eqEvalsSIMDVars {
			weightedTerm := api.ExtensionMul(
				values[i*simdSize+j],
				mpiEval,
				simdEval,
			)
			res = api.ExtensionAdd(res, weightedTerm)
		}
	}

	return res
}

func eqTerm(
	api fields.ArithmeticEngine,
	x, y []frontend.Variable,
) []frontend.Variable {

	// (xy + (1 - x)(1 - y)) = 2xy + 1 - x - y
	res := api.ExtensionMul(x, y, api.ToExtension(2))
	res[0] = api.Add(res[0], 1)

	res = api.ExtensionSub(res, x, y)

	return res
}

// EqVec computes \prod eq(x_i, y_i), where eq(x_i, y_i) stands for
// (x_i y_i) + (1 - x_i) (1 - y_i).
func EqVec(
	api fields.ArithmeticEngine,
	xs, ys [][]frontend.Variable,
) []frontend.Variable {

	res := api.One()

	for i := 0; i < len(xs); i++ {
		eqAtI := eqTerm(api, xs[i], ys[i])
		res = api.ExtensionMul(eqAtI, res)
	}

	return res
}
