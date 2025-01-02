package verifier

import (
	"ExpanderVerifierCircuit/modules/circuit"
	"ExpanderVerifierCircuit/modules/fields"

	"github.com/consensys/gnark/frontend"
)

func PrepareLayer(
	api fields.ArithmeticEngine,
	layer *circuit.Layer,
	alpha []frontend.Variable,
	rz0, rz1, r_simd, r_mpi [][]frontend.Variable,
	sp *ScratchPad,
	is_output_layer bool,
) {
	if is_output_layer {
		EqEvalsAtEfficient(
			api,
			rz0,
			api.One(),
			sp.EqEvalsAtRz0,
			sp.EqEvalsFirstPart,
			sp.EqEvalsSecondPart,
			sp.EqEvalsCount,
		)
	} else {
		output_len := 1 << len(rz0)
		copy(sp.EqEvalsAtRz0[:output_len], sp.EqEvalsAtRx[:output_len])
		if rz1 != nil && alpha != nil {
			for i := 0; i < 1<<layer.OutputLenLog; i++ {
				sp.EqEvalsAtRz0[i] = api.ExtensionAdd(
					sp.EqEvalsAtRz0[i],
					api.ExtensionMul(alpha, sp.EqEvalsAtRy[i]),
				)
			}
		}
	}

	EqEvalsAtEfficient(
		api,
		r_simd,
		api.One(),
		sp.EqEvalsAtRSimd,
		sp.EqEvalsFirstPart,
		sp.EqEvalsSecondPart,
		sp.EqEvalsCount,
	)

	EqEvalsAtEfficient(
		api,
		r_mpi,
		api.One(),
		sp.EqEvalsAtRMpi,
		sp.EqEvalsFirstPart,
		sp.EqEvalsSecondPart,
		sp.EqEvalsCount,
	)

	sp.RSimd = r_simd
	sp.RMpi = r_mpi
}

func EvalCst(
	api fields.ArithmeticEngine,
	cst_gates []circuit.Gate,
	public_input [][]frontend.Variable,
	sp *ScratchPad,
) []frontend.Variable {

	v := api.Zero()

	var mpi_size = len(sp.EqEvalsAtRMpi)
	var simd_size = len(sp.EqEvalsAtRSimd)

	for i := 0; i < len(cst_gates); i++ {
		var cst_gate circuit.Gate = cst_gates[i]

		tmp := api.Zero()

		switch cst_gate.Coef.CoefType {
		case circuit.PublicInput:
			n_witnesses := len(public_input)
			if n_witnesses != mpi_size*simd_size {
				panic("Incompatible n_witnesses with mpi and simd size")
			}
			input_idx := cst_gate.Coef.InputIdx
			vals := make([][]frontend.Variable, n_witnesses)
			for j := 0; j < n_witnesses; j++ {
				vals[j] = api.ToExtension(public_input[j][input_idx])
			}

			tmp = CombineWithSimdMpi(api, vals, sp.EqEvalsAtRSimd, sp.EqEvalsAtRMpi)
			tmp = api.ExtensionMul(tmp, sp.EqEvalsAtRz0[cst_gate.OId])
		default:
			coef_value := api.ToExtension(cst_gate.Coef.GetActualLocalValue())
			tmp = api.ExtensionMul(sp.EqEvalsAtRz0[cst_gate.OId], coef_value)
		}

		v = api.ExtensionAdd(v, tmp)
	}
	return v
}

func EvalAdd(
	api fields.ArithmeticEngine,
	add_gates []circuit.Gate,
	sp *ScratchPad,
) []frontend.Variable {

	v := api.Zero()

	for i := 0; i < len(add_gates); i++ {
		var add_gate = add_gates[i]
		v = api.ExtensionAdd(
			v,
			api.ExtensionMul(
				sp.EqEvalsAtRz0[add_gate.OId],
				sp.EqEvalsAtRx[add_gate.IIds[0]],
				api.ToExtension(add_gate.Coef.GetActualLocalValue()),
			),
		)
	}
	return api.ExtensionMul(v, sp.EqRSimdRSimdXY, sp.EqRMpiRMpiXY)
}

func EvalMul(
	api fields.ArithmeticEngine,
	mul_gates []circuit.Gate,
	sp *ScratchPad,
) []frontend.Variable {

	v := api.Zero()

	for i := 0; i < len(mul_gates); i++ {
		var mul_gate = mul_gates[i]
		v = api.ExtensionAdd(
			v,
			api.ExtensionMul(
				sp.EqEvalsAtRz0[mul_gate.OId],
				sp.EqEvalsAtRx[mul_gate.IIds[0]],
				sp.EqEvalsAtRy[mul_gate.IIds[1]],
				api.ToExtension(mul_gate.Coef.GetActualLocalValue()),
			),
		)
	}
	return api.ExtensionMul(v, sp.EqRSimdRSimdXY, sp.EqRMpiRMpiXY)
}

func SetRx(
	api fields.ArithmeticEngine,
	rx [][]frontend.Variable,
	sp *ScratchPad,
) {
	EqEvalsAtEfficient(
		api,
		rx,
		api.One(),
		sp.EqEvalsAtRx,
		sp.EqEvalsFirstPart,
		sp.EqEvalsSecondPart,
		sp.EqEvalsCount,
	)
}

func SetRSimdXY(
	api fields.ArithmeticEngine,
	r_simd_xy [][]frontend.Variable,
	sp *ScratchPad,
) {
	sp.EqRSimdRSimdXY = EqVec(api, sp.RSimd, r_simd_xy)
}

func SetRMPIXY(
	api fields.ArithmeticEngine,
	r_mpi_xy [][]frontend.Variable,
	sp *ScratchPad,
) {
	sp.EqRMpiRMpiXY = EqVec(api, sp.RMpi, r_mpi_xy)
}

func SetRY(
	api fields.ArithmeticEngine,
	r_y [][]frontend.Variable,
	sp *ScratchPad,
) {
	EqEvalsAtEfficient(
		api,
		r_y,
		api.One(),
		sp.EqEvalsAtRy,
		sp.EqEvalsFirstPart,
		sp.EqEvalsSecondPart,
		sp.EqEvalsCount,
	)
}

func Degree2Eval(
	api fields.ArithmeticEngine,
	lagrangeEvals [][]frontend.Variable, // lagrange evals at x=0, 1, 2
	evalPoint []frontend.Variable,
	sp ScratchPad,
) []frontend.Variable {
	c0 := lagrangeEvals[0]

	c2 := api.ExtensionSub(
		api.ExtensionAdd(lagrangeEvals[2], lagrangeEvals[0]),
		lagrangeEvals[1],
		lagrangeEvals[1],
	)
	c2 = api.ExtensionMul(c2, api.ToExtension(sp.Inv2))

	c1 := api.ExtensionSub(lagrangeEvals[1], lagrangeEvals[0], c2)

	return api.ExtensionAdd(
		api.ExtensionMul(
			api.ExtensionAdd(api.ExtensionMul(c2, evalPoint), c1),
			evalPoint,
		),
		c0,
	)
}

func Degree3Eval(
	api fields.ArithmeticEngine,
	lagrangeEvals [][]frontend.Variable, // lagrange evals at x=0, 1, 2, 3
	evalPoint []frontend.Variable,
	sp ScratchPad,
) []frontend.Variable {
	return LagEval(api, lagrangeEvals, evalPoint, sp)
}

func LagEval(
	api fields.ArithmeticEngine,
	lagrangeEvals [][]frontend.Variable,
	evalPoint []frontend.Variable,
	sp ScratchPad,
) []frontend.Variable {
	res := api.Zero()

	for i := range lagrangeEvals {
		lagEval := api.One()

		for j := range lagrangeEvals {
			if j == i {
				continue
			}
			lagEval = api.ExtensionMul(
				lagEval,
				api.ExtensionSub(evalPoint, api.ToExtension(sp.Deg3EvalAt[j])),
			)
		}

		lagEval = api.ExtensionMul(
			lagEval,
			lagrangeEvals[i],
			api.ToExtension(sp.Deg3LagDenomsInv[i]),
		)
		res = api.ExtensionAdd(res, lagEval)
	}

	return res
}
