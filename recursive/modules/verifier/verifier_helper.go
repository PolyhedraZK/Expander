package verifier

import (
	"ExpanderVerifierCircuit/modules/circuit"

	"github.com/consensys/gnark/frontend"
)

func PrepareLayer(
	api frontend.API,
	layer *circuit.Layer,
	alpha frontend.Variable,
	beta frontend.Variable,
	rz0 []frontend.Variable,
	rz1 []frontend.Variable,
	r_simd []frontend.Variable,
	r_mpi []frontend.Variable,
	sp *ScratchPad,
) {
	EqEvalsAtEfficient(
		api,
		rz0,
		alpha,
		sp.EqEvalsAtRz0,
		sp.EqEvalsFirstPart,
		sp.EqEvalsSecondPart,
	)

	if rz1 != nil && beta != nil {
		EqEvalsAtEfficient(
			api,
			rz1,
			beta,
			sp.EqEvalsAtRz1,
			sp.EqEvalsFirstPart,
			sp.EqEvalsSecondPart,
		)

		for i := 0; i < 1<<layer.OutputLenLog; i++ {
			sp.EqEvalsAtRz0[i] = api.Add(sp.EqEvalsAtRz0[i], sp.EqEvalsAtRz1[i])
		}
	}

	EqEvalsAtEfficient(
		api,
		r_simd,
		1,
		sp.EqEvalsAtRSimd,
		sp.EqEvalsFirstPart,
		sp.EqEvalsSecondPart,
	)

	EqEvalsAtEfficient(
		api,
		r_mpi,
		1,
		sp.EqEvalsAtRMpi,
		sp.EqEvalsFirstPart,
		sp.EqEvalsSecondPart,
	)

	sp.RSimd = &r_simd
	sp.RMpi = &r_mpi
}

func EvalCst(
	api frontend.API,
	cst_gates []circuit.Gate,
	public_input [][]frontend.Variable,
	sp *ScratchPad,
) frontend.Variable {
	var v frontend.Variable = 0

	var mpi_size = len(sp.EqEvalsAtRMpi)
	var simd_size = len(sp.EqEvalsAtRSimd)

	if mpi_size != 1 || simd_size != 1 {
		panic("Only support mpi size 1 and simd size 1 for now")
	}

	for i := 0; i < len(cst_gates); i++ {
		var cst_gate circuit.Gate = cst_gates[i]

		var tmp frontend.Variable = 0
		switch cst_gate.Coef.CoefType {
		case circuit.PublicInput:
			n_witnesses := len(public_input)
			if n_witnesses != mpi_size*simd_size {
				panic("Incompatible n_witnesses with mpi and simd size")
			}
			input_idx := cst_gate.Coef.InputIdx
			vals := make([]frontend.Variable, n_witnesses)
			for j := 0; j < n_witnesses; j++ {
				vals[j] = public_input[j][input_idx]
			}

			tmp = CombineWithSimdMpi(api, vals, sp.EqEvalsAtRSimd, sp.EqEvalsAtRMpi)
			tmp = api.Mul(tmp, sp.EqEvalsAtRz0[cst_gate.OId])
		default:
			coef_value := cst_gate.Coef.GetActualLocalValue()
			tmp = api.Mul(sp.EqEvalsAtRz0[cst_gate.OId], coef_value)
		}

		v = api.Add(v, tmp)
	}
	return v
}

func EvalAdd(
	api frontend.API,
	add_gates []circuit.Gate,
	sp *ScratchPad,
) frontend.Variable {
	var v frontend.Variable = 0
	for i := 0; i < len(add_gates); i++ {
		var add_gate = add_gates[i]
		v = api.Add(
			v,
			api.Mul(sp.EqEvalsAtRz0[add_gate.OId], sp.EqEvalsAtRx[add_gate.IIds[0]], add_gate.Coef.GetActualLocalValue()),
		)
	}
	return api.Mul(v, sp.EqRSimdRSimdXY, sp.EqRMpiRMpiXY)
}

func EvalMul(
	api frontend.API,
	mul_gates []circuit.Gate,
	sp *ScratchPad,
) frontend.Variable {
	var v frontend.Variable = 0
	for i := 0; i < len(mul_gates); i++ {
		var mul_gate = mul_gates[i]
		v = api.Add(
			v,
			api.Mul(
				sp.EqEvalsAtRz0[mul_gate.OId],
				sp.EqEvalsAtRx[mul_gate.IIds[0]],
				sp.EqEvalsAtRy[mul_gate.IIds[1]],
				mul_gate.Coef.GetActualLocalValue(),
			),
		)
	}
	return api.Mul(v, sp.EqRSimdRSimdXY, sp.EqRMpiRMpiXY)
}

func SetRx(
	api frontend.API,
	rx []frontend.Variable,
	sp *ScratchPad,
) {
	EqEvalsAtEfficient(
		api,
		rx,
		1,
		sp.EqEvalsAtRx,
		sp.EqEvalsFirstPart,
		sp.EqEvalsSecondPart,
	)
}

func SetRSimdXY(
	api frontend.API,
	r_simd_xy []frontend.Variable,
	sp *ScratchPad,
) {
	sp.EqRSimdRSimdXY = EqVec(api, *sp.RSimd, r_simd_xy)
}

func SetRMPIXY(
	api frontend.API,
	r_mpi_xy []frontend.Variable,
	sp *ScratchPad,
) {
	sp.EqRMpiRMpiXY = EqVec(api, *sp.RMpi, r_mpi_xy)
}

func SetRY(
	api frontend.API,
	r_y []frontend.Variable,
	sp *ScratchPad,
) {
	EqEvalsAtEfficient(
		api,
		r_y,
		1,
		sp.EqEvalsAtRy,
		sp.EqEvalsFirstPart,
		sp.EqEvalsSecondPart,
	)
}

func Degree2Eval(
	api frontend.API,
	p []frontend.Variable, // lag vals at x=0, 1, 2
	x frontend.Variable,
	sp *ScratchPad,
) frontend.Variable {
	var c0 = p[0]
	var c2 = api.Mul(
		sp.Inv2,
		api.Sub(api.Add(p[2], p[0]), p[1], p[1]),
	)
	var c1 = api.Sub(p[1], p[0], c2)
	return api.Add(
		api.Mul(
			api.Add(api.Mul(c2, x), c1),
			x,
		),
		c0,
	)
}

func Degree3Eval(
	api frontend.API,
	p []frontend.Variable, // lag vals at x=0, 1, 2, 3
	x frontend.Variable,
	sp *ScratchPad,
) frontend.Variable {
	return LagEval(api, p, x, sp)
}

func LagEval(
	api frontend.API,
	vals []frontend.Variable,
	x frontend.Variable,
	sp *ScratchPad,
) frontend.Variable {
	if len(vals) != len(sp.Deg3EvalAt) {
		panic("Incorrect length in LagEval")
	}

	var v frontend.Variable = 0
	for i := 0; i < len(vals); i++ {
		var numerator frontend.Variable = 0
		for j := 0; j < len(vals); j++ {
			if j == i {
				continue
			}

			numerator = api.Mul(numerator, api.Sub(x, sp.Deg3EvalAt[j]))
		}
		v = api.Add(v, api.Mul(numerator, sp.Deg3LagDenomsInv[i], vals[i]))
	}
	return v
}
