package verifier

import (
	"github.com/consensys/gnark/frontend"
)

func EqEvalsAtPrimitive(
	api frontend.API,
	r []frontend.Variable,
	mul_factor frontend.Variable,
	ret_evals []frontend.Variable,
) {
	ret_evals[0] = mul_factor
	var cur_eval_num = 1

	for i := 0; i < len(r); i++ {
		for j := 0; j < cur_eval_num; j++ {
			ret_evals[j+cur_eval_num] = api.Mul(ret_evals[j], r[i])
			ret_evals[j] = api.Sub(ret_evals[j], ret_evals[j+cur_eval_num])
		}
		cur_eval_num <<= 1
	}
}

func EqEvalsAtEfficient(
	api frontend.API,
	r []frontend.Variable,
	mul_factor frontend.Variable,
	ret_evals []frontend.Variable,
	tmp_1st_half []frontend.Variable,
	tmp_2nd_half []frontend.Variable,
	eq_evals_count map[uint]uint,
) {
	ret_len := uint(1) << len(r)
	if val, ok := eq_evals_count[ret_len]; ok {
		eq_evals_count[ret_len] = val + 1
	} else {
		eq_evals_count[ret_len] = 1
	}

	var first_half_bits uint = uint(len(r) >> 1)
	var first_half_mask uint = (1 << first_half_bits) - 1

	EqEvalsAtPrimitive(api, r[:first_half_bits], mul_factor, tmp_1st_half)
	EqEvalsAtPrimitive(api, r[first_half_bits:], 1, tmp_2nd_half)

	for i := uint(0); i < (1 << len(r)); i++ {
		var first_half = i & first_half_mask
		var second_half = i >> first_half_bits
		ret_evals[i] = api.Mul(tmp_1st_half[first_half], tmp_2nd_half[second_half])
	}
}

func CombineWithSimdMpi(
	api frontend.API,
	values []frontend.Variable,
	eq_evals_at_simd []frontend.Variable,
	eq_evals_at_mpi []frontend.Variable,
) frontend.Variable {
	var mpi_size = len(eq_evals_at_mpi)
	var simd_size = len(eq_evals_at_simd)

	var r frontend.Variable = 0
	for i := 0; i < mpi_size; i++ {
		for j := 0; j < simd_size; j++ {
			var idx = (i*simd_size + j)
			r = api.Add(r, api.Mul(values[idx], eq_evals_at_simd[j], eq_evals_at_mpi[i]))
		}
	}
	return r
}

func Eq(api frontend.API, x frontend.Variable, y frontend.Variable) frontend.Variable {
	var xy = api.Mul(x, y)
	return api.Sub(api.Add(xy, xy, 1), x, y)
}

func EqVec(api frontend.API, x []frontend.Variable, y []frontend.Variable) frontend.Variable {
	var r frontend.Variable = 1
	for i := 0; i < len(x); i++ {
		r = api.Mul(r, Eq(api, x[i], y[i]))
	}
	return r
}
